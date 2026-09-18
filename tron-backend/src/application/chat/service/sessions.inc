impl ChatService {
    pub fn create_session(
        &self,
        command: CreateChatSessionCommand,
        user_code: Option<String>,
    ) -> Result<ChatSession, ChatError> {
        let workspace_code =
            normalized_required(command.workspace_code).ok_or(ChatError::WorkspaceRequired)?;
        let id = self
            .id_provider
            .next_id()
            .map_err(|_| ChatError::StorageFailure)?
            .value();
        let now = current_timestamp();
        let task_code = normalized_optional(command.task_code);
        let source_type = command.source_type.unwrap_or(if task_code.is_some() {
            ChatSessionSourceType::Task
        } else {
            ChatSessionSourceType::Manual
        });
        let project_code = normalized_optional(command.project_code);
        let app_code = if source_type == ChatSessionSourceType::Manual
            && self.projects.is_some()
            && project_code.is_some()
        {
            let projects = self.projects.as_ref().ok_or(ChatError::ProjectInvalid)?;
            let project = projects
                .resolve_execution_context(
                    &workspace_code,
                    project_code.as_deref().expect("project code is checked"),
                )
                .map_err(|_| ChatError::ProjectInvalid)?;
            Some(project.primary_app.app_code)
        } else {
            normalized_optional(command.app_code)
        };
        let session = ChatSession {
            id,
            session_code: format!("SESSION-{id}"),
            workspace_code,
            project_code,
            user_code,
            app_code,
            task_code,
            source_type,
            source_code: normalized_optional(command.source_code),
            agent_code: normalized_optional(command.agent_code),
            system_agent_session_id: None,
            title: normalized_optional(command.title)
                .unwrap_or_else(|| DEFAULT_MANUAL_SESSION_TITLE.into()),
            status: ChatSessionStatus::Active,
            last_run_code: None,
            last_run_status: None,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut repository = self.repository();
        if session.source_type == ChatSessionSourceType::Manual
            && session.project_code.is_some()
            && let Some(existing) = repository.list_sessions()?.into_iter().find(|existing| {
                existing.workspace_code == session.workspace_code
                    && existing.project_code == session.project_code
                    && existing.user_code == session.user_code
                    && existing.source_type == ChatSessionSourceType::Manual
                    && existing.status == ChatSessionStatus::Active
                    && existing.last_run_code.is_none()
            })
        {
            return Ok(existing);
        }
        repository.create_session(&session)?;
        Ok(session)
    }

    pub fn list_session_groups(
        &self,
        query: ChatSessionQuery,
    ) -> Result<Vec<ChatSessionGroup>, ChatError> {
        let workspace_code =
            normalized_required(query.workspace_code).ok_or(ChatError::WorkspaceRequired)?;
        let mut sessions = self.repository().list_sessions()?;
        sessions.retain(|session| {
            session.workspace_code == workspace_code
                && session.status == ChatSessionStatus::Active
                && session.source_type != ChatSessionSourceType::Terminal
        });

        let mut grouped = BTreeMap::<String, ChatSessionGroup>::new();
        for session in sessions {
            let task_title = match session.source_type {
                ChatSessionSourceType::Automation => "Automation".into(),
                ChatSessionSourceType::Task => {
                    session.task_code.clone().unwrap_or_else(|| "Task".into())
                }
                ChatSessionSourceType::Manual => "Chat".into(),
                ChatSessionSourceType::Terminal => "Terminal".into(),
            };
            let group_key = format!(
                "{:?}:{}:{}",
                session.source_type,
                session.source_code.as_deref().unwrap_or(""),
                session.task_code.as_deref().unwrap_or("")
            );
            let group = grouped
                .entry(group_key)
                .or_insert_with(|| ChatSessionGroup {
                    task_code: session.task_code.clone(),
                    task_title,
                    source_type: session.source_type,
                    source_code: session.source_code.clone(),
                    pending_user_input_count: 0,
                    pending_user_input_session_codes: Vec::new(),
                    sessions: Vec::new(),
                });
            group.sessions.push(session);
        }
        for group in grouped.values_mut() {
            for session in &group.sessions {
                let pending = self
                    .repository()
                    .list_user_input_requests(&session.session_code)?
                    .into_iter()
                    .filter(|request| request.status == ChatUserInputRequestStatus::Pending)
                    .count() as u32;
                if pending > 0 {
                    group.pending_user_input_count += pending;
                    group
                        .pending_user_input_session_codes
                        .push(session.session_code.clone());
                }
            }
        }
        Ok(grouped.into_values().collect())
    }

    pub fn get_session(&self, session_code: &str) -> Result<ChatSession, ChatError> {
        self.repository().get_session(session_code)
    }

    pub fn archive_session(&self, session_code: &str) -> Result<ChatSession, ChatError> {
        let updated_at = current_timestamp();
        self.repository().update_session_status(
            session_code,
            ChatSessionStatus::Archived,
            &updated_at,
        )?;
        self.repository().get_session(session_code)
    }

    pub fn delete_session(&self, session_code: &str) -> Result<ChatSession, ChatError> {
        let session = self.repository().get_session(session_code)?;
        if let Some(resources) = self.resources.as_ref() {
            resources
                .delete_chat_session_attachments(session_code)
                .map_err(chat_error_from_resource)?;
        }
        self.repository().delete_session(session_code)?;
        Ok(session)
    }

    pub fn store_session_attachment_from_reader(
        &self,
        session_code: &str,
        command: ChatSessionAttachmentUploadCommand,
        source: &mut dyn std::io::Read,
    ) -> Result<ChatMessageAttachment, ChatError> {
        let session = self.repository().get_session(session_code)?;
        let resources = self.resources.as_ref().ok_or(ChatError::StorageFailure)?;
        let id = self
            .id_provider
            .next_id()
            .map_err(|_| ChatError::StorageFailure)?
            .value();
        let attachment_code = format!("CHAT-ATTACH-{id}");
        let summary =
            normalized_optional(command.summary).unwrap_or_else(|| command.file_name.clone());
        let attachment = ChatMessageAttachment {
            id: attachment_code.clone(),
            kind: command.kind,
            title: Some(command.file_name.clone()),
            summary,
            image_url: None,
            resource_code: Some(attachment_code.clone()),
            file_name: Some(command.file_name.clone()),
            content_type: Some(command.content_type.clone()),
            byte_size: None,
            metadata: command.metadata,
        };
        let mut attachment = normalize_chat_attachments(vec![attachment])?
            .pop()
            .expect("a single normalized attachment is retained");
        let record = resources
            .store_chat_session_attachment(
                ChatSessionAttachmentCreateCommand {
                    workspace_code: session.workspace_code,
                    session_code: session.session_code,
                    attachment_code,
                    file_name: command.file_name,
                    content_type: command.content_type,
                },
                source,
            )
            .map_err(chat_error_from_resource)?;
        apply_attachment_record(&mut attachment, &record);
        Ok(attachment)
    }

    pub fn read_session_attachment(
        &self,
        session_code: &str,
        attachment_code: &str,
    ) -> Result<(ChatSessionAttachmentRecord, Vec<u8>), ChatError> {
        let session = self.repository().get_session(session_code)?;
        let resources = self.resources.as_ref().ok_or(ChatError::StorageFailure)?;
        resources
            .read_chat_session_attachment(
                &session.workspace_code,
                &session.session_code,
                attachment_code,
            )
            .map_err(chat_error_from_resource)
    }

    pub fn update_session(
        &self,
        session_code: &str,
        command: UpdateChatSessionCommand,
    ) -> Result<ChatSession, ChatError> {
        let title = normalized_required(command.title).ok_or(ChatError::SessionTitleRequired)?;
        let updated_at = current_timestamp();
        self.repository()
            .update_session_title(session_code, &title, &updated_at)?;
        self.repository().get_session(session_code)
    }

    pub fn list_messages(&self, session_code: &str) -> Result<Vec<ChatMessage>, ChatError> {
        self.repository().get_session(session_code)?;
        self.repository().list_messages(session_code)
    }

    pub fn list_user_input_requests(
        &self,
        session_code: &str,
    ) -> Result<Vec<ChatUserInputRequest>, ChatError> {
        self.repository().get_session(session_code)?;
        self.repository().list_user_input_requests(session_code)
    }

    pub fn answer_user_input(
        &self,
        session_code: &str,
        request_code: &str,
        answers: Vec<ChatUserInputAnswer>,
    ) -> Result<SendChatMessageCommand, ChatError> {
        let session = self.repository().get_session(session_code)?;
        let request = self
            .repository()
            .list_user_input_requests(session_code)?
            .into_iter()
            .find(|item| item.request_code == request_code)
            .ok_or(ChatError::UserInputRequestNotFound)?;
        if request.status != ChatUserInputRequestStatus::Pending {
            return Err(ChatError::UserInputAlreadyAnswered);
        }
        validate_user_input_answers(&request.questions, &answers)?;
        let responses = request.questions.iter().filter_map(|question| {
            let answer = answers.iter().find(|answer| answer.question_code == question.question_code)?;
            let answer_text = match answer.answer_kind {
                ChatUserInputAnswerKind::Option => answer.option_code.as_deref()
                    .and_then(|code| question.options.iter().find(|option| option.option_code == code))
                    .map(|option| option.label.clone())
                    .or_else(|| answer.option_code.clone())
                    .unwrap_or_default(),
                ChatUserInputAnswerKind::Other => answer.other_text.clone().unwrap_or_default(),
                ChatUserInputAnswerKind::Skipped => "Skipped".into(),
            };
            Some(json!({ "question": question.question, "answer": answer_text }))
        }).collect::<Vec<_>>();
        let answered_at = current_timestamp();
        let answered = self.repository().answer_user_input_request(
            session_code,
            request_code,
            &answers,
            &answered_at,
        )?;
        let model_selection = Self::continuation_model_selection(answered.model_code.as_deref());
        Ok(SendChatMessageCommand {
            run_code: None,
            content: None,
            workspace_code: Some(session.workspace_code),
            app_code: session.app_code,
            task_code: session.task_code,
            agent_code: session.agent_code,
            model_selection,
            model_code: answered.model_code,
            system_model_id: None,
            reasoning_effort: answered.reasoning_effort,
            limits: None,
            tools: Vec::new(),
            attachments: Vec::new(),
            client_capabilities: Some(crate::domain::chat::ChatClientCapabilities {
                user_input_requests: true,
            }),
            resume: vec![crate::application::chat::ChatResumeEntry {
                interrupt_id: request_code.to_string(),
                status: "resolved".into(),
                payload: json!({ "answers": answers, "responses": responses }),
            }],
        })
    }

    fn continuation_model_selection(model_code: Option<&str>) -> Option<ModelSelection> {
        let model_code = model_code?;
        if model_code == "PERSON_AI:ROUTE:ChatAuto" {
            return Some(ModelSelection::auto());
        }
        let fixed = model_code.strip_prefix("PERSON_AI:FIXED:")?;
        let (quality, catalog_model_code) = fixed.split_once(':')?;
        Some(ModelSelection::person_ai_fixed(
            catalog_model_code,
            ProviderModelQuality::from_storage(quality)?,
        ))
    }

    pub fn steer_message(
        &self,
        command: SteerChatMessageCommand,
    ) -> Result<ChatSteerResult, ChatSteerError> {
        let session_code =
            normalized_required(command.session_code).ok_or(ChatSteerError::SessionCodeRequired)?;
        let expected_run_code = normalized_required(command.expected_run_code)
            .ok_or(ChatSteerError::ExpectedRunCodeRequired)?;
        let content = normalized_optional(command.content).unwrap_or_default();
        let attachments =
            normalize_chat_attachments(command.attachments).map_err(ChatSteerError::Chat)?;
        if content.is_empty() && attachments.is_empty() {
            return Err(ChatSteerError::Chat(ChatError::MessageRequired));
        }
        let session = self
            .repository()
            .get_session(&session_code)
            .map_err(ChatSteerError::Chat)?;
        let (attachments, invocation_attachments) = self
            .prepare_session_attachments(&session, attachments)
            .map_err(ChatSteerError::Chat)?;
        let prompt = agent_prompt_for_content(&content);
        let channel = if session.task_code.is_some() {
            ChatMessageChannel::TaskAgent
        } else {
            ChatMessageChannel::MainAgent
        };

        match self
            .active_runs
            .accept_steer(&session_code, &expected_run_code, || {
                let message = self.create_message(
                    &session_code,
                    Some(expected_run_code.clone()),
                    ChatMessageRole::User,
                    channel,
                    "User",
                    &content,
                    attachments.clone(),
                    None,
                )?;
                let input = AgentSteerInput {
                    steer_code: message.message_code.clone(),
                    content: prompt.clone(),
                    attachments: invocation_attachments.clone(),
                };
                Ok::<_, ChatSendError>((
                    ChatSteerResult {
                        session_code: session_code.clone(),
                        run_code: expected_run_code.clone(),
                        message,
                    },
                    input,
                ))
            }) {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(ChatSendError::Chat(error))) => Err(ChatSteerError::Chat(error)),
            Ok(Err(ChatSendError::Agent(_) | ChatSendError::RunAlreadyActive { .. })) => {
                Err(ChatSteerError::RegistryUnavailable)
            }
            Err(error) => Err(ChatSteerError::from(error)),
        }
    }
}
