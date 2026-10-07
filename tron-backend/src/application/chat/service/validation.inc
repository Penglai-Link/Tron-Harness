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

fn resume_prompt(resume: &[ChatResumeEntry]) -> String {
    if let Some(summary) = resume_answer_summary(resume) {
        return format!(
            "Continue the interrupted task using the user's answers below. Treat them as the user's response to the questions you asked, then continue the task without asking the same questions again unless a new ambiguity blocks progress.\n\n{summary}"
        );
    }
    let payload = resume
        .iter()
        .map(|entry| {
            json!({
                "interruptId": entry.interrupt_id,
                "status": entry.status,
                "payload": entry.payload,
            })
        })
        .collect::<Vec<_>>();
    format!(
        "Resume the interrupted task using these user responses: {}",
        json!(payload)
    )
}

fn resume_answer_summary(resume: &[ChatResumeEntry]) -> Option<String> {
    let responses = resume
        .iter()
        .flat_map(|entry| {
            entry
                .payload
                .get("responses")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|response| {
            let question = response.get("question")?.as_str()?.trim();
            let answer = response.get("answer")?.as_str()?.trim();
            (!question.is_empty()).then(|| {
                format!(
                    "- {question}\n  Answer: {}",
                    if answer.is_empty() { "Skipped" } else { answer }
                )
            })
        })
        .collect::<Vec<_>>();
    (!responses.is_empty()).then(|| responses.join("\n"))
}

fn resume_user_message(resume: &[ChatResumeEntry]) -> Option<String> {
    resume_answer_summary(resume)
        .map(|summary| format!("The user answered the questions you asked:\n{summary}"))
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
    #[serde(default)]
    pub(super) framework_interaction_id: Option<String>,
    pub(super) intro: String,
    questions: Vec<RawUserInputQuestion>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserInputQuestion {
    id: String,
    question: String,
    options: Vec<RawUserInputOption>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserInputOption {
    id: String,
    label: String,
    description: Option<String>,
}

/// Question cards are projections of a native paused run.
/// Generic Input and Approval retain their full schema and content on `run.native`.
pub(super) fn paused_run_presentation(
    run: &AgentInvocationResult,
) -> Result<(String, Option<RawUserInputRequest>), ChatSendError> {
    if let Some(native) = run
        .native
        .as_ref()
        .filter(|native| native.status == crate::domain::agent::AgentNativeRunStatus::AwaitingInput)
    {
        if native.interactions.is_empty() {
            return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "native paused run has no pending interaction".into(),
            )));
        }
        for interaction in &native.interactions {
            if let Some(presentation) = interaction.presentation.as_ref().filter(|value| {
                value.get("kind").and_then(serde_json::Value::as_str) == Some("ask_user")
            }) {
                let mut raw: RawUserInputRequest = serde_json::from_value(presentation.clone())
                    .map_err(|_| {
                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                            "native question presentation is invalid".into(),
                        ))
                    })?;
                raw.framework_interaction_id = Some(interaction.id.clone());
                return Ok((raw.intro.clone(), Some(raw)));
            }
        }
        return Ok((run.final_output.clone(), None));
    }
    Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
        "paused result must contain a native Framework interaction".into(),
    )))
}
#[cfg(test)]
mod native_pause_tests {
    include!("native_pause_tests.inc");
}
