use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    application::{
        agent::{
            AgentInvocationCommand, AgentInvocationError, AgentInvocationService, ModelStreamSink,
        },
        chat::{
            ChatRepository, ChatSendResult, ChatSessionQuery, CreateChatSessionCommand,
            SendChatMessageCommand, UpdateChatSessionCommand,
        },
        llm::LlmRuntimeService,
    },
    domain::agent::{AgentModelConfig, ModelTokenUsage},
    domain::chat::{
        ChatError, ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession, ChatSessionGroup,
        ChatSessionSourceType, ChatSessionStatus,
    },
    shared::{
        id::{SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

use super::title::{DEFAULT_MANUAL_SESSION_TITLE, should_generate_manual_session_title};

const MAIN_AGENT_CODE: &str = "main-agent";
const MAIN_RUNTIME_ADAPTER_KEY: &str = "REACT";
const CODEX_RUNTIME_ADAPTER_KEY: &str = "CODEX_EXEC";
const OPENCODE_RUNTIME_ADAPTER_KEY: &str = "OPENCODE_RUN";
static SESSION_RUN_COORDINATOR: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static CHAT_SERVICE_COORDINATOR_NAMESPACE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<Mutex<Box<dyn ChatRepository>>>,
    id_provider: Arc<dyn SnowflakeIdProvider>,
    coordinator_namespace: u64,
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
            id_provider,
            coordinator_namespace: CHAT_SERVICE_COORDINATOR_NAMESPACE
                .fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn clone_with_id_provider(&self, id_provider: Arc<dyn SnowflakeIdProvider>) -> Self {
        Self {
            repository: self.repository.clone(),
            id_provider,
            coordinator_namespace: self.coordinator_namespace,
        }
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
            session.workspace_code == workspace_code && session.status == ChatSessionStatus::Active
        });

        let mut grouped = BTreeMap::<String, ChatSessionGroup>::new();
        for session in sessions {
            let task_title = match session.source_type {
                ChatSessionSourceType::Automation => "Automation".into(),
                ChatSessionSourceType::Task => {
                    session.task_code.clone().unwrap_or_else(|| "Task".into())
                }
                ChatSessionSourceType::Manual => "Chat".into(),
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
                    sessions: Vec::new(),
                });
            group.sessions.push(session);
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
        self.repository().delete_session(session_code)?;
        Ok(session)
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
        let content = normalized_required(command.content)
            .ok_or(ChatSendError::Chat(ChatError::MessageRequired))?;
        let session = self
            .repository()
            .get_session(session_code)
            .map_err(ChatSendError::Chat)?;
        let should_generate_title = should_generate_manual_session_title(&session);
        let _session_run_guard = acquire_session_run(self.coordinator_namespace, session_code)?;
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
        let runtime_adapter_key = runtime_adapter_key_for_agent(&effective_agent_code);
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
            None,
        )?;
        let app_code_for_invocation = command.app_code.or(session.app_code.clone());
        let task_code_for_invocation = command.task_code.or(session.task_code.clone());
        let system_model_id_for_invocation = command.system_model_id;
        let limits_for_invocation = command.limits;
        let tools_for_invocation = command.tools;
        let invocation = AgentInvocationCommand {
            run_code: command.run_code,
            prompt: Some(content.clone()),
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
            transcript_input_kind: None,
            transcript_input_metadata: None,
        };
        let run = if let Some(sink) = sink {
            agent
                .invoke_stream(invocation, user_code.clone(), locale.clone(), sink)
                .await
        } else {
            agent
                .invoke(invocation, user_code.clone(), locale.clone())
                .await
        }
        .map_err(ChatSendError::Agent)?;
        let final_output = run.final_output.clone();
        let assistant_message = self.create_message(
            session_code,
            Some(run.run_code.clone()),
            ChatMessageRole::Assistant,
            channel,
            "Assistant",
            &run.final_output,
            run.token_usage,
        )?;
        let mut result_messages = vec![user_message, assistant_message];
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
        if should_generate_title && let Some(llm_runtime) = llm_runtime {
            self.try_generate_manual_session_title(
                session_code,
                &workspace_code,
                &content,
                &final_output,
                llm_runtime,
            )
            .await;
        }
        if let Some(short_term_memory) = agent.short_term_memory() {
            let goal_messages = self
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
                )
                .await?;
            result_messages.extend(goal_messages);
        }
        let updated_session = self
            .repository()
            .get_session(session_code)
            .map_err(ChatSendError::Chat)?;
        Ok(ChatSendResult {
            session: updated_session,
            run,
            messages: result_messages,
        })
    }

    pub(super) fn create_message(
        &self,
        session_code: &str,
        run_code: Option<String>,
        role: ChatMessageRole,
        channel: ChatMessageChannel,
        actor: &str,
        content: &str,
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
        return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
            "session already has an active run".into(),
        )));
    }
    Ok(SessionRunGuard { run_key })
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_required(value: Option<String>) -> Option<String> {
    normalized_optional(value)
}

fn runtime_adapter_key_for_agent(agent_code: &str) -> &'static str {
    match agent_code.trim().to_ascii_lowercase().as_str() {
        "codex" => CODEX_RUNTIME_ADAPTER_KEY,
        "opencode" => OPENCODE_RUNTIME_ADAPTER_KEY,
        _ => MAIN_RUNTIME_ADAPTER_KEY,
    }
}
