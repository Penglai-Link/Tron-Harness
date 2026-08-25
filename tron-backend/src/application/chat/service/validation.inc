#[derive(Debug, Clone, PartialEq)]
pub enum ChatSendError {
    Agent(AgentInvocationError),
    Chat(ChatError),
    RunAlreadyActive { active_run_code: Option<String> },
}

struct SessionRunGuard {
    run_key: String,
}

impl Drop for SessionRunGuard {
    fn drop(&mut self) {
        if let Some(coordinator) = SESSION_RUN_COORDINATOR.get()
            && let Ok(mut active_sessions) = coordinator.lock()
        {
            active_sessions.remove(&self.run_key);
        }
    }
}

fn acquire_session_run(
    coordinator_namespace: u64,
    session_code: &str,
) -> Result<SessionRunGuard, ChatSendError> {
    let coordinator = SESSION_RUN_COORDINATOR.get_or_init(|| Mutex::new(BTreeSet::new()));
    let mut active_sessions = coordinator.lock().map_err(|_| {
        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
            "goal run coordinator is unavailable".into(),
        ))
    })?;
    let run_key = format!("{coordinator_namespace}:{session_code}");
    if !active_sessions.insert(run_key.clone()) {
        return Err(ChatSendError::RunAlreadyActive {
            active_run_code: None,
        });
    }
    Ok(SessionRunGuard { run_key })
}

fn apply_attachment_record(
    attachment: &mut ChatMessageAttachment,
    record: &ChatSessionAttachmentRecord,
) {
    attachment.id = record.attachment_code.clone();
    attachment.resource_code = Some(record.attachment_code.clone());
    attachment.title = Some(record.file_name.clone());
    attachment.file_name = Some(record.file_name.clone());
    attachment.content_type = Some(record.content_type.clone());
    attachment.byte_size = Some(record.byte_size);
    attachment.image_url = None;
}

fn hydrate_attachment_for_agent(
    attachment: &mut ChatMessageAttachment,
    record: &ChatSessionAttachmentRecord,
    bytes: &[u8],
) {
    if bytes.len() <= MAX_AGENT_ATTACHMENT_IMAGE_BYTES
        && is_supported_agent_image(&record.content_type)
    {
        attachment.image_url = Some(format!(
            "data:{};base64,{}",
            record.content_type,
            BASE64.encode(bytes),
        ));
    }
    if let Some(text) = attachment_text_excerpt(record, bytes) {
        let metadata = std::mem::take(&mut attachment.metadata);
        attachment.metadata = match metadata {
            Value::Object(mut object) => {
                object.insert("__chatAttachmentText".to_string(), Value::String(text));
                Value::Object(object)
            }
            metadata => json!({
                "__chatAttachmentText": text,
                "metadata": metadata,
            }),
        };
    }
}

fn is_supported_agent_image(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    )
}

fn attachment_text_excerpt(record: &ChatSessionAttachmentRecord, bytes: &[u8]) -> Option<String> {
    let content_type = record.content_type.to_ascii_lowercase();
    let text_like_content_type = content_type.starts_with("text/")
        || matches!(
            content_type.as_str(),
            "application/json"
                | "application/ld+json"
                | "application/xml"
                | "application/x-yaml"
                | "application/yaml"
        );
    let text_like_extension = record
        .file_name
        .rsplit_once('.')
        .map(|(_, extension)| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "txt"
                    | "md"
                    | "markdown"
                    | "json"
                    | "csv"
                    | "tsv"
                    | "log"
                    | "yaml"
                    | "yml"
                    | "xml"
                    | "html"
                    | "htm"
                    | "rs"
                    | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "py"
                    | "java"
                    | "go"
                    | "sql"
                    | "sh"
                    | "toml"
            )
        })
        .unwrap_or(false);
    if !text_like_content_type && !text_like_extension {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let excerpt = text
        .chars()
        .take(MAX_AGENT_ATTACHMENT_TEXT_CHARS)
        .collect::<String>();
    (!excerpt.trim().is_empty()).then_some(excerpt)
}

fn agent_prompt_for_content(content: &str) -> String {
    if content.trim().is_empty() {
        "Please review the attached file(s).".to_string()
    } else {
        content.to_string()
    }
}

fn chat_error_from_resource(error: ResourceError) -> ChatError {
    match error {
        ResourceError::NotFound | ResourceError::MissingBinaryObject => {
            ChatError::AttachmentNotFound
        }
        ResourceError::InvalidCode
        | ResourceError::InvalidInput
        | ResourceError::InvalidField { .. }
        | ResourceError::InvalidVariableReference
        | ResourceError::PathDenied
        | ResourceError::StorageLocatorInvalid => ChatError::AttachmentInvalid(error.to_string()),
        ResourceError::Forbidden
        | ResourceError::WorkspaceRequired
        | ResourceError::InvalidWorkspace
        | ResourceError::Conflict
        | ResourceError::StorageFailure
        | ResourceError::StorageProfileNotFound
        | ResourceError::ConnectionVerificationFailed(_)
        | ResourceError::ServerHasAccounts => ChatError::StorageFailure,
    }
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_required(value: Option<String>) -> Option<String> {
    normalized_optional(value)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawUserInputRequest {
    pub(super) intro: String,
    questions: Vec<RawUserInputQuestion>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserInputQuestion {
    question: String,
    options: Vec<RawUserInputOption>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserInputOption {
    label: String,
    description: Option<String>,
}

pub(super) fn raw_user_input_request(
    run: &AgentInvocationResult,
) -> Result<RawUserInputRequest, ChatSendError> {
    run.observations
        .iter()
        .rev()
        .find(|observation| observation.tool_name == crate::application::agent::CHAT_ASK_USER_TOOL)
        .and_then(|observation| {
            serde_json::from_str::<RawUserInputRequest>(&observation.output).ok()
        })
        .ok_or_else(|| {
            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "chat.ask_user returned an invalid request".into(),
            ))
        })
}

fn validate_user_input_answers(
    questions: &[ChatUserInputQuestion],
    answers: &[ChatUserInputAnswer],
) -> Result<(), ChatError> {
    if answers.len() != questions.len() {
        return Err(ChatError::UserInputAnswerInvalid(
            "one answer is required for every question".into(),
        ));
    }
    for question in questions {
        let Some(answer) = answers
            .iter()
            .find(|answer| answer.question_code == question.question_code)
        else {
            return Err(ChatError::UserInputAnswerInvalid(format!(
                "missing answer for question `{}`",
                question.question_code
            )));
        };
        match answer.answer_kind {
            ChatUserInputAnswerKind::Option => {
                let Some(option_code) = answer.option_code.as_deref() else {
                    return Err(ChatError::UserInputAnswerInvalid(
                        "option answer requires optionCode".into(),
                    ));
                };
                if !question
                    .options
                    .iter()
                    .any(|option| option.option_code == option_code)
                {
                    return Err(ChatError::UserInputAnswerInvalid(format!(
                        "option `{option_code}` does not belong to the question"
                    )));
                }
                if answer
                    .other_text
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    return Err(ChatError::UserInputAnswerInvalid(
                        "option answer cannot include otherText".into(),
                    ));
                }
            }
            ChatUserInputAnswerKind::Other => {
                let Some(other_text) = answer.other_text.as_deref() else {
                    return Err(ChatError::UserInputAnswerInvalid(
                        "other answer requires otherText".into(),
                    ));
                };
                if other_text.trim().is_empty() || other_text.chars().count() > 2_000 {
                    return Err(ChatError::UserInputAnswerInvalid(
                        "otherText must contain 1 to 2000 characters".into(),
                    ));
                }
                if answer.option_code.is_some() {
                    return Err(ChatError::UserInputAnswerInvalid(
                        "other answer cannot include optionCode".into(),
                    ));
                }
            }
            ChatUserInputAnswerKind::Skipped => {
                if answer.option_code.is_some() || answer.other_text.is_some() {
                    return Err(ChatError::UserInputAnswerInvalid(
                        "skipped answer cannot include optionCode or otherText".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn format_user_input_answer_summary(
    questions: &[ChatUserInputQuestion],
    answers: &[ChatUserInputAnswer],
) -> String {
    let lines = questions
        .iter()
        .filter_map(|question| {
            let answer = answers
                .iter()
                .find(|answer| answer.question_code == question.question_code)?;
            let value = match answer.answer_kind {
                ChatUserInputAnswerKind::Option => question
                    .options
                    .iter()
                    .find(|option| {
                        answer.option_code.as_deref() == Some(option.option_code.as_str())
                    })
                    .map(|option| option.label.clone())
                    .unwrap_or_else(|| "(invalid option)".into()),
                ChatUserInputAnswerKind::Other => answer
                    .other_text
                    .clone()
                    .unwrap_or_else(|| "(empty other answer)".into()),
                ChatUserInputAnswerKind::Skipped => "Skipped".into(),
            };
            Some(format!("{}: {}", question.question, value))
        })
        .collect::<Vec<_>>();
    format!(
        "The user answered the questions you asked:\n{}\nContinue the task using these answers.",
        lines.join("\n")
    )
}
