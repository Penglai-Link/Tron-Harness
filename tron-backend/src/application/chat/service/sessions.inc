impl ChatService {
    /// Trusted host entrypoint. Platform Dude ownership is persisted by DudeService;
    /// public user session creation continues to require an actual workspace.
    pub fn ensure_dude_session(&self, dude: &crate::domain::dude::DudeSession, user_code: Option<String>) -> Result<ChatSession, ChatError> {
        if let Ok(existing)=self.repository().get_session(&dude.session_code) { if existing.user_code!=user_code {return Err(ChatError::WorkspaceForbidden);} return Ok(existing); }
        let now=current_timestamp();
        let session=ChatSession {
            id:self.id_provider.next_id().map_err(|_|ChatError::StorageFailure)?.value(),
            session_code:dude.session_code.clone(), workspace_code:dude.workspace_code.clone().unwrap_or_default(),
            project_code:None,user_code,app_code:None,task_code:(dude.purpose == crate::domain::dude::DudeSessionPurpose::Task).then(|| dude.correlation_key.clone()),source_type:ChatSessionSourceType::Manual,
            source_code:Some(format!("__dude:{}",dude.dude_code)),agent_code:Some(dude.dude_code.clone()),
            system_agent_session_id:None,title:dude.title.clone(),status:ChatSessionStatus::Active,
            last_run_code:None,last_run_status:None,created_at:now.clone(),updated_at:now,
        };
        let mut repository=self.repository();
        if let Ok(existing)=repository.get_session(&dude.session_code){if existing.user_code!=session.user_code {return Err(ChatError::WorkspaceForbidden);}return Ok(existing)}
        repository.create_session(&session)?;Ok(session)
    }

    pub fn create_session(
        &self,
        command: CreateChatSessionCommand,
        user_code: Option<String>,
    ) -> Result<ChatSession, ChatError> {
        if command.source_code.as_deref().is_some_and(|source| source.starts_with("__dude:")) {
            return Err(ChatError::WorkspaceForbidden);
        }
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
            project.primary_app.map(|app| app.app_code)
        } else {
            normalized_optional(command.app_code)
        };
        let session = ChatSession {
            id,
            session_code: eliteforge_agent_core::SessionRecord::new("").id,
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
                && !session.source_code.as_deref().is_some_and(|s|s.starts_with("__dude:"))
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

    pub fn session_titles(&self, codes: &[String]) -> Result<Vec<crate::application::chat::ChatSessionTitle>, ChatError> {
        let codes = codes.iter().filter(|code| !code.is_empty()).cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        if codes.is_empty() { return Ok(Vec::new()); }
        self.repository().session_titles(&codes)
    }

    pub fn ensure_session_writable(&self, session_code: &str) -> Result<(), ChatError> {
        let session = self.repository().get_session(session_code)?;
        if session.status != ChatSessionStatus::Active
            || self
                .active_runs
                .is_retired(session_code)
                .map_err(|_| ChatError::StorageFailure)?
        {
            return Err(ChatError::SessionArchived);
        }
        Ok(())
    }

    pub fn archive_session(&self, session_code: &str) -> Result<ChatSession, ChatError> {
        let session = self.repository().get_session(session_code)?;
        if self
            .execution_memory
            .as_ref()
            .map(|memory| memory.get_active_goal(session_code))
            .transpose()
            .map_err(|_| ChatError::StorageFailure)?
            .flatten()
            .is_some_and(|goal| goal.goal.status == "ACTIVE")
        {
            return Err(ChatError::SessionRunActive);
        }
        self.active_runs
            .retire_session(session_code)
            .map_err(|error| {
                if matches!(
                    error,
                    crate::application::agent::ActiveRunError::AlreadyActive
                ) {
                    ChatError::SessionRunActive
                } else {
                    ChatError::StorageFailure
                }
            })?;
        if session.status == ChatSessionStatus::Archived {
            return Ok(session);
        }
        let updated_at = current_timestamp();
        if let Err(error) = self.repository().update_session_status(
            session_code,
            ChatSessionStatus::Archived,
            &updated_at,
        ) {
            let _ = self.active_runs.reactivate_session(session_code);
            return Err(error);
        }
        self.repository().get_session(session_code)
    }

    pub fn restore_session(&self, session_code: &str) -> Result<ChatSession, ChatError> {
        let session = self.repository().get_session(session_code)?;
        if session.status == ChatSessionStatus::Active {
            self.active_runs
                .reactivate_session(session_code)
                .map_err(|_| ChatError::StorageFailure)?;
            return Ok(session);
        }
        let updated_at = current_timestamp();
        self.repository().update_session_status(
            session_code,
            ChatSessionStatus::Active,
            &updated_at,
        )?;
        self.active_runs
            .reactivate_session(session_code)
            .map_err(|_| ChatError::StorageFailure)?;
        self.repository().get_session(session_code)
    }

    pub fn list_archived_sessions(
        &self,
        workspace_code: Option<&str>,
        current: u32,
        size: u32,
    ) -> Result<(Vec<ChatSession>, u64), ChatError> {
        let current = current.max(1);
        let size = size.clamp(1, 100);
        let offset = current.saturating_sub(1).saturating_mul(size);
        self.repository()
            .list_archived_sessions(workspace_code, offset, size)
    }

    pub fn prepare_session_deletion(&self, session_code: &str) -> Result<(), ChatError> {
        let session = self.repository().get_session(session_code)?;
        self.ensure_archived_session_inactive(session_code, &session)
    }

    pub fn delete_session(&self, session_code: &str) -> Result<ChatSession, ChatError> {
        let session = self.repository().get_session(session_code)?;
        self.ensure_archived_session_inactive(session_code, &session)?;
        let attachments = self
            .resources
            .as_ref()
            .map(|resources| resources.chat_session_attachment_records(session_code))
            .transpose()
            .map_err(chat_error_from_resource)?
            .unwrap_or_default();
        self.repository().delete_session(session_code)?;
        if let Some(resources) = self.resources.as_ref() {
            resources.cleanup_chat_session_attachment_files(&attachments);
        }
        Ok(session)
    }

    fn ensure_archived_session_inactive(
        &self,
        session_code: &str,
        session: &ChatSession,
    ) -> Result<(), ChatError> {
        if session.status != ChatSessionStatus::Archived {
            return Err(ChatError::SessionMustBeArchived);
        }
        if self
            .execution_memory
            .as_ref()
            .map(|memory| memory.get_active_goal(session_code))
            .transpose()
            .map_err(|_| ChatError::StorageFailure)?
            .flatten()
            .is_some_and(|goal| goal.goal.status == "ACTIVE")
        {
            return Err(ChatError::SessionRunActive);
        }
        self.active_runs
            .retire_session(session_code)
            .map_err(|error| {
                if matches!(
                    error,
                    crate::application::agent::ActiveRunError::AlreadyActive
                ) {
                    ChatError::SessionRunActive
                } else {
                    ChatError::StorageFailure
                }
            })?;
        Ok(())
    }

    pub fn store_session_attachment_from_reader(
        &self,
        session_code: &str,
        command: ChatSessionAttachmentUploadCommand,
        source: &mut (dyn std::io::Read + Send),
    ) -> Result<ChatMessageAttachment, ChatError> {
        let session = self.repository().get_session(session_code)?;
        if session.status != ChatSessionStatus::Active {
            return Err(ChatError::SessionArchived);
        }
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
        let memory = self
            .execution_memory
            .as_ref()
            .ok_or(ChatError::StorageFailure)?;
        let mut stored = None;
        let result = memory.with_active_session_write(session_code, || {
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
            stored = Some(record.clone());
            Ok(record)
        });
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                if let Some(record) = stored {
                    resources.cleanup_chat_session_attachment_files(&[record]);
                }
                return Err(error);
            }
        };
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
        let session = self.repository().get_session(session_code)?;
        if session.status != ChatSessionStatus::Active {
            return Err(ChatError::SessionArchived);
        }
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

    /// Product session/model mapping only; native schema and answer CAS live in the SDK.
    pub fn native_resume_command(
        &self,
        session_code: &str,
        entry: crate::application::chat::ChatResumeEntry,
        model_code: Option<String>,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    ) -> Result<SendChatMessageCommand, ChatError> {
        let session = self.repository().get_session(session_code)?;
        Ok(SendChatMessageCommand {
            annotations: Vec::new(),
            requested_skills: Vec::new(),
            a2ui_action_id: None, a2ui_metadata: None,
            run_code: None,
            content: None,
            workspace_code: Some(session.workspace_code),
            app_code: session.app_code,
            task_code: session.task_code,
            agent_code: session.agent_code,
            model_selection: Self::continuation_model_selection(model_code.as_deref()),
            model_code,
            system_model_id: None,
            reasoning_effort,
            page_context: None,
            limits: None,
            tools: Vec::new(),
            attachments: Vec::new(),
            client_capabilities: Some(crate::domain::chat::ChatClientCapabilities {
                user_input_requests: true,
            a2ui: None,
            }),
            resume: vec![entry],
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
        eliteforge_agent_core::validate_annotations(&command.annotations).map_err(|_| ChatSteerError::Chat(ChatError::AnnotationsInvalid))?;
        let annotations = command.annotations;
        let content = normalized_optional(command.content).unwrap_or_default();
        let attachments =
            normalize_chat_attachments(command.attachments).map_err(ChatSteerError::Chat)?;
        if content.is_empty() && attachments.is_empty() && annotations.is_empty() {
            return Err(ChatSteerError::Chat(ChatError::MessageRequired));
        }
        let session = self
            .repository()
            .get_session(&session_code)
            .map_err(ChatSteerError::Chat)?;
        if session.status != ChatSessionStatus::Active { return Err(ChatSteerError::Chat(ChatError::SessionArchived)); }
        self.repository().validate_annotation_sources(&session_code, &annotations).map_err(ChatSteerError::Chat)?;
        let agent_code = session.agent_code.as_deref().unwrap_or(MAIN_AGENT_CODE);
        let requested_skills = self.authorize_requested_skills(agent_code, &command.requested_skills).map_err(ChatSteerError::Chat)?;
        let goal = super::composer_input::goal_objective(&content).map_err(ChatSteerError::Chat)?;
        if goal.is_some() { self.ensure_goal_agent(agent_code).map_err(ChatSteerError::Chat)?; }
        let (attachments, invocation_attachments) = self
            .prepare_session_attachments(&session, attachments)
            .map_err(ChatSteerError::Chat)?;
        let prompt = if !annotations.is_empty() { content.clone() } else { agent_prompt_for_content(&content) };
        let channel = if session.task_code.is_some() {
            ChatMessageChannel::TaskAgent
        } else {
            ChatMessageChannel::MainAgent
        };

        match self
            .active_runs
            .accept_steer(&session_code, &expected_run_code, || {
                // Admission is fenced by the active-run lock before command side effects.
                if let Some(objective) = goal { self.apply_goal_command(&session_code, objective, crate::shared::auth_holder::AuthHolder::get().map(|auth| auth.user_code().to_string())).map_err(ChatSendError::Chat)?; }
                let message = self.create_message_with_skills(
                    &session_code,
                    Some(expected_run_code.clone()),
                    ChatMessageRole::User,
                    channel,
                    "User",
                    &content,
                    attachments.clone(),
                    None,
                    requested_skills.clone(),
                    annotations.clone(),
                )?;
                let input = AgentSteerInput {
                    annotations: annotations.clone(),
                    steer_code: message.message_code.clone(),
                    requested_skills: requested_skills.clone(),
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
