use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

use crate::{
    application::{
        agent::{
            ActiveRunRegistry, AgentInvocationCommand, AgentInvocationError, AgentInvocationResult,
            AgentInvocationService, AgentSteerInput, ModelStreamSink,
        },
        chat::{
            ChatRepository, ChatSendResult, ChatSessionAttachmentUploadCommand, ChatSessionQuery,
            ChatSteerError, ChatSteerResult, CreateChatSessionCommand, SendChatMessageCommand,
            SteerChatMessageCommand, UpdateChatSessionCommand,
        },
        llm::LlmRuntimeService,
        resource::{ChatSessionAttachmentCreateCommand, ResourceService},
    },
    domain::agent::{AgentModelConfig, ModelTokenUsage},
    domain::chat::{
        ChatError, ChatMessage, ChatMessageAttachment, ChatMessageChannel, ChatMessageRole,
        ChatSession, ChatSessionGroup, ChatSessionSourceType, ChatSessionStatus,
        ChatUserInputAnswer, ChatUserInputAnswerKind, ChatUserInputOption, ChatUserInputQuestion,
        ChatUserInputRequest, ChatUserInputRequestStatus,
    },
    domain::resource::{ChatSessionAttachmentRecord, ResourceError},
    shared::{
        id::{SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

use super::{
    attachments::normalize_chat_attachments,
    title::{DEFAULT_MANUAL_SESSION_TITLE, should_generate_manual_session_title},
};

const MAIN_AGENT_CODE: &str = "main-agent";
const MAX_AGENT_ATTACHMENT_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_AGENT_ATTACHMENT_TEXT_CHARS: usize = 256 * 1024;
static SESSION_RUN_COORDINATOR: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static CHAT_SERVICE_COORDINATOR_NAMESPACE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<Mutex<Box<dyn ChatRepository>>>,
    resources: Option<ResourceService>,
    id_provider: Arc<dyn SnowflakeIdProvider>,
    coordinator_namespace: u64,
    pub(super) active_runs: ActiveRunRegistry,
}

pub(super) struct GoalContinuationResult {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) user_input_request: Option<ChatUserInputRequest>,
}

impl ChatService {
    pub fn new(repository: impl ChatRepository + 'static) -> Self {
        Self::with_id_provider(
            repository,
            Arc::new(SnowflakedSnowflakeIdProvider::default()),
        )
    }

    pub fn with_id_provider(
        repository: impl ChatRepository + 'static,
        id_provider: Arc<dyn SnowflakeIdProvider>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            resources: None,
            id_provider,
            coordinator_namespace: CHAT_SERVICE_COORDINATOR_NAMESPACE
                .fetch_add(1, Ordering::Relaxed),
            active_runs: ActiveRunRegistry::new(),
        }
    }

    pub fn clone_with_id_provider(&self, id_provider: Arc<dyn SnowflakeIdProvider>) -> Self {
        Self {
            repository: self.repository.clone(),
            resources: self.resources.clone(),
            id_provider,
            coordinator_namespace: self.coordinator_namespace,
            active_runs: self.active_runs.clone(),
        }
    }

    pub fn with_active_runs(mut self, active_runs: ActiveRunRegistry) -> Self {
        self.active_runs = active_runs;
        self
    }

    pub fn with_resources(mut self, resources: ResourceService) -> Self {
        self.resources = Some(resources);
        self
    }

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
        let session = ChatSession {
            id,
            session_code: format!("SESSION-{id}"),
            workspace_code,
            user_code,
            app_code: normalized_optional(command.app_code),
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
        let answered_at = current_timestamp();
        let answered = self.repository().answer_user_input_request(
            session_code,
            request_code,
            &answers,
            &answered_at,
        )?;
        let summary = format_user_input_answer_summary(&answered.questions, &answers);
        Ok(SendChatMessageCommand {
            run_code: None,
            content: Some(summary),
            workspace_code: Some(session.workspace_code),
            app_code: session.app_code,
            task_code: session.task_code,
            agent_code: session.agent_code,
            model_code: answered.model_code,
            system_model_id: None,
            reasoning_effort: answered.reasoning_effort,
            limits: None,
            tools: Vec::new(),
            attachments: Vec::new(),
            client_capabilities: Some(crate::domain::chat::ChatClientCapabilities {
                user_input_requests: true,
            }),
        })
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

    pub async fn send_message(
        &self,
        session_code: &str,
        command: SendChatMessageCommand,
        resolved_model: AgentModelConfig,
        user_code: Option<String>,
        locale: Option<String>,
        llm_runtime: Option<LlmRuntimeService>,
        agent: AgentInvocationService,
    ) -> Result<ChatSendResult, ChatSendError> {
        self.send_message_inner(
            session_code,
            command,
            resolved_model,
            user_code,
            locale,
            llm_runtime,
            agent,
            None,
        )
        .await
    }

    pub async fn send_message_stream(
        &self,
        session_code: &str,
        command: SendChatMessageCommand,
        resolved_model: AgentModelConfig,
        user_code: Option<String>,
        locale: Option<String>,
        llm_runtime: Option<LlmRuntimeService>,
        agent: AgentInvocationService,
        sink: ModelStreamSink,
    ) -> Result<ChatSendResult, ChatSendError> {
        self.send_message_inner(
            session_code,
            command,
            resolved_model,
            user_code,
            locale,
            llm_runtime,
            agent,
            Some(sink),
        )
        .await
    }

    async fn send_message_inner(
        &self,
        session_code: &str,
        command: SendChatMessageCommand,
        resolved_model: AgentModelConfig,
        user_code: Option<String>,
        locale: Option<String>,
        llm_runtime: Option<LlmRuntimeService>,
        agent: AgentInvocationService,
        sink: Option<ModelStreamSink>,
    ) -> Result<ChatSendResult, ChatSendError> {
        let content = normalized_optional(command.content).unwrap_or_default();
        let attachments =
            normalize_chat_attachments(command.attachments).map_err(ChatSendError::Chat)?;
        if content.is_empty() && attachments.is_empty() {
            return Err(ChatSendError::Chat(ChatError::MessageRequired));
        }
        let session = self
            .repository()
            .get_session(session_code)
            .map_err(ChatSendError::Chat)?;
        let (attachments, invocation_attachments) = self
            .prepare_session_attachments(&session, attachments)
            .map_err(ChatSendError::Chat)?;
        let prompt = agent_prompt_for_content(&content);
        let should_generate_title = should_generate_manual_session_title(&session);
        let workspace_code = session.workspace_code.clone();
        let channel = if command
            .task_code
            .as_ref()
            .or(session.task_code.as_ref())
            .is_some()
        {
            ChatMessageChannel::TaskAgent
        } else {
            ChatMessageChannel::MainAgent
        };
        let effective_agent_code = if session.last_run_code.is_some() {
            session.agent_code.clone()
        } else {
            normalized_optional(command.agent_code.clone()).or(session.agent_code.clone())
        }
        .unwrap_or_else(|| MAIN_AGENT_CODE.to_string());
        let reservation = self.reserve_run(
            session_code,
            command.run_code.clone(),
            &effective_agent_code,
        )?;
        let run_code = reservation.run_code;
        let runtime_adapter_key = reservation.runtime_adapter_key;
        let active_run_reservation = reservation.guard;
        let _session_run_guard = acquire_session_run(self.coordinator_namespace, session_code)?;
        let mapped_runtime_session_id = self
            .repository()
            .runtime_session_id(session_code, runtime_adapter_key, &effective_agent_code)
            .map_err(ChatSendError::Chat)?;
        let user_message = self.create_message(
            session_code,
            None,
            ChatMessageRole::User,
            channel,
            "User",
            &content,
            attachments.clone(),
            None,
        )?;
        if should_generate_title
            && !content.is_empty()
            && let Some(llm_runtime) = llm_runtime
        {
            let chat_service = self.clone();
            let session_code = session_code.to_string();
            let workspace_code = workspace_code.clone();
            let user_message_content = content.clone();
            tokio::spawn(async move {
                chat_service
                    .try_generate_manual_session_title(
                        &session_code,
                        &workspace_code,
                        &user_message_content,
                        llm_runtime,
                    )
                    .await;
            });
        }
        let app_code_for_invocation = command.app_code.or(session.app_code.clone());
        let task_code_for_invocation = command.task_code.or(session.task_code.clone());
        let supports_user_input_requests = command
            .client_capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.user_input_requests);
        let system_model_id_for_invocation = command.system_model_id;
        let limits_for_invocation = command.limits;
        let tools_for_invocation = command.tools;
        let invocation = AgentInvocationCommand {
            run_code: Some(run_code),
            prompt: Some(prompt),
            workspace_code: Some(workspace_code.clone()),
            conversation_code: Some(session.session_code.clone()),
            agent_code: Some(effective_agent_code.clone()),
            system_agent_session_id: mapped_runtime_session_id
                .or(session.system_agent_session_id.clone()),
            app_code: app_code_for_invocation.clone(),
            task_code: task_code_for_invocation.clone(),
            model: Some(resolved_model.clone()),
            system_model_id: system_model_id_for_invocation.clone(),
            limits: limits_for_invocation,
            tools: tools_for_invocation.clone(),
            attachments: invocation_attachments,
            supports_user_input_requests,
            transcript_input_kind: None,
            transcript_input_metadata: None,
        };
        let run = if let Some(sink) = sink {
            agent
                .invoke_stream_reserved(
                    invocation,
                    user_code.clone(),
                    locale.clone(),
                    sink,
                    active_run_reservation,
                )
                .await
        } else {
            agent
                .invoke_reserved(
                    invocation,
                    user_code.clone(),
                    locale.clone(),
                    active_run_reservation,
                )
                .await
        }
        .map_err(ChatSendError::Agent)?;
        let awaiting_user_input = run.status == "AWAITING_USER_INPUT";
        let (assistant_content, user_input_request) = if awaiting_user_input {
            let raw = raw_user_input_request(&run)?;
            (raw.intro.clone(), Some(raw))
        } else {
            (run.final_output.clone(), None)
        };
        let assistant_message = self.create_message(
            session_code,
            Some(run.run_code.clone()),
            ChatMessageRole::Assistant,
            channel,
            "Assistant",
            &assistant_content,
            Vec::new(),
            run.token_usage,
        )?;
        let mut result_messages = vec![user_message, assistant_message.clone()];
        let mut user_input_request = if let Some(raw) = user_input_request {
            Some(self.persist_user_input_request(session_code, &run, &assistant_message, raw)?)
        } else {
            None
        };
        let updated_at = current_timestamp();
        if let Some(runtime_session_id) = run.system_agent_session_id.as_deref() {
            self.repository()
                .upsert_runtime_session(
                    session_code,
                    run.runtime_adapter_key.as_str(),
                    &effective_agent_code,
                    runtime_session_id,
                    Some(run.run_code.as_str()),
                    &updated_at,
                )
                .map_err(ChatSendError::Chat)?;
        }
        self.repository()
            .update_session_run(
                session_code,
                &run.run_code,
                run.status,
                Some(effective_agent_code.as_str()),
                run.system_agent_session_id.as_deref(),
                &updated_at,
            )
            .map_err(ChatSendError::Chat)?;
        if !awaiting_user_input && let Some(short_term_memory) = agent.short_term_memory() {
            let goal_result = self
                .run_goal_continuations(
                    session_code,
                    &session,
                    &effective_agent_code,
                    app_code_for_invocation,
                    task_code_for_invocation,
                    system_model_id_for_invocation,
                    resolved_model,
                    tools_for_invocation,
                    user_code,
                    locale,
                    agent,
                    short_term_memory,
                    run.clone(),
                    run.system_agent_session_id.clone(),
                    supports_user_input_requests,
                )
                .await?;
            result_messages.extend(goal_result.messages);
            user_input_request = user_input_request.or(goal_result.user_input_request);
        }
        let updated_session = self
            .repository()
            .get_session(session_code)
            .map_err(ChatSendError::Chat)?;
        Ok(ChatSendResult {
            session: updated_session,
            run,
            messages: result_messages,
            user_input_request,
        })
    }

    pub(super) fn persist_user_input_request(
        &self,
        session_code: &str,
        run: &AgentInvocationResult,
        assistant_message: &ChatMessage,
        raw: RawUserInputRequest,
    ) -> Result<ChatUserInputRequest, ChatSendError> {
        let id = self
            .id_provider
            .next_id()
            .map_err(|_| ChatSendError::Chat(ChatError::StorageFailure))?
            .value();
        let questions = raw
            .questions
            .into_iter()
            .enumerate()
            .map(|(index, question)| ChatUserInputQuestion {
                question_code: format!("QUESTION-{id}-{index}"),
                question: question.question,
                options: question
                    .options
                    .into_iter()
                    .enumerate()
                    .map(|(option_index, option)| ChatUserInputOption {
                        option_code: format!("OPTION-{id}-{index}-{option_index}"),
                        label: option.label,
                        description: option.description,
                    })
                    .chain([
                        ChatUserInputOption {
                            option_code: format!("OTHER-{id}-{index}"),
                            label: "Other".into(),
                            description: Some("Provide a custom answer.".into()),
                        },
                        ChatUserInputOption {
                            option_code: format!("SKIP-{id}-{index}"),
                            label: "Skip".into(),
                            description: Some("Skip this question.".into()),
                        },
                    ])
                    .collect(),
            })
            .collect::<Vec<_>>();
        if questions.is_empty() {
            return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "chat.ask_user returned no questions".into(),
            )));
        }
        let request = ChatUserInputRequest {
            request_code: format!("INPUT-{id}"),
            session_code: session_code.to_string(),
            run_code: run.run_code.clone(),
            assistant_message_code: Some(assistant_message.message_code.clone()),
            intro: raw.intro,
            questions,
            answers: None,
            model_code: Some(run.model_code.clone()),
            reasoning_effort: run.reasoning_effort,
            status: ChatUserInputRequestStatus::Pending,
            requested_at: current_timestamp(),
            answered_at: None,
        };
        self.repository()
            .create_user_input_request(&request)
            .map_err(ChatSendError::Chat)?;
        Ok(request)
    }

    fn prepare_session_attachments(
        &self,
        session: &ChatSession,
        attachments: Vec<ChatMessageAttachment>,
    ) -> Result<(Vec<ChatMessageAttachment>, Vec<ChatMessageAttachment>), ChatError> {
        if attachments
            .iter()
            .all(|attachment| attachment.resource_code.is_none())
        {
            return Ok((attachments.clone(), attachments));
        }
        let resources = self.resources.as_ref().ok_or(ChatError::StorageFailure)?;
        let mut persisted = Vec::with_capacity(attachments.len());
        let mut invocation = Vec::with_capacity(attachments.len());
        for mut attachment in attachments {
            let Some(attachment_code) = attachment.resource_code.clone() else {
                invocation.push(attachment.clone());
                persisted.push(attachment);
                continue;
            };
            let (record, bytes) = resources
                .read_chat_session_attachment(
                    &session.workspace_code,
                    &session.session_code,
                    &attachment_code,
                )
                .map_err(chat_error_from_resource)?;
            apply_attachment_record(&mut attachment, &record);
            let mut agent_attachment = attachment.clone();
            hydrate_attachment_for_agent(&mut agent_attachment, &record, &bytes);
            persisted.push(attachment);
            invocation.push(agent_attachment);
        }
        Ok((persisted, invocation))
    }

    pub(super) fn create_message(
        &self,
        session_code: &str,
        run_code: Option<String>,
        role: ChatMessageRole,
        channel: ChatMessageChannel,
        actor: &str,
        content: &str,
        attachments: Vec<ChatMessageAttachment>,
        token_usage: Option<ModelTokenUsage>,
    ) -> Result<ChatMessage, ChatSendError> {
        let id = self
            .id_provider
            .next_id()
            .map_err(|_| ChatSendError::Chat(ChatError::StorageFailure))?
            .value();
        let message = ChatMessage {
            id,
            message_code: format!("MSG-{id}"),
            session_code: session_code.to_string(),
            run_code,
            role,
            channel,
            actor: actor.into(),
            content: content.into(),
            attachments,
            input_tokens: token_usage.map(|usage| usage.input_tokens),
            output_tokens: token_usage.map(|usage| usage.output_tokens),
            total_tokens: token_usage.map(|usage| usage.total_tokens),
            created_at: current_timestamp(),
        };
        let mut repository = self.repository();
        repository
            .create_message(&message)
            .map_err(ChatSendError::Chat)?;
        Ok(message)
    }

    pub(super) fn repository(&self) -> std::sync::MutexGuard<'_, Box<dyn ChatRepository>> {
        self.repository
            .lock()
            .expect("chat repository mutex poisoned")
    }
}

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
