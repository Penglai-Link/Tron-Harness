use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
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
        llm::{LlmRuntimeService, LlmTextGenerationCallCommand},
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

const MAIN_AGENT_CODE: &str = "main-agent";
const MAIN_RUNTIME_ADAPTER_KEY: &str = "REACT";
const CODEX_RUNTIME_ADAPTER_KEY: &str = "CODEX_EXEC";
const OPENCODE_RUNTIME_ADAPTER_KEY: &str = "OPENCODE_RUN";
const DEFAULT_MANUAL_SESSION_TITLE: &str = "New chat";
const MAX_GENERATED_TITLE_CHARS: usize = 64;

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<Mutex<Box<dyn ChatRepository>>>,
    id_provider: Arc<dyn SnowflakeIdProvider>,
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
        let invocation = AgentInvocationCommand {
            run_code: command.run_code,
            prompt: Some(content.clone()),
            workspace_code: Some(workspace_code.clone()),
            conversation_code: Some(session.session_code.clone()),
            agent_code: Some(effective_agent_code.clone()),
            system_agent_session_id: mapped_runtime_session_id
                .or(session.system_agent_session_id.clone()),
            app_code: command.app_code.or(session.app_code.clone()),
            task_code: command.task_code.or(session.task_code.clone()),
            model: Some(resolved_model),
            system_model_id: command.system_model_id,
            limits: command.limits,
            tools: command.tools,
        };
        let run = if let Some(sink) = sink {
            agent
                .invoke_stream(invocation, user_code, locale.clone(), sink)
                .await
        } else {
            agent.invoke(invocation, user_code, locale).await
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
        let updated_session = self
            .repository()
            .get_session(session_code)
            .map_err(ChatSendError::Chat)?;
        Ok(ChatSendResult {
            session: updated_session,
            run,
            messages: vec![user_message, assistant_message],
        })
    }

    fn create_message(
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

    fn repository(&self) -> std::sync::MutexGuard<'_, Box<dyn ChatRepository>> {
        self.repository
            .lock()
            .expect("chat repository mutex poisoned")
    }

    async fn try_generate_manual_session_title(
        &self,
        session_code: &str,
        workspace_code: &str,
        user_message: &str,
        assistant_message: &str,
        llm_runtime: LlmRuntimeService,
    ) {
        let prompt = manual_session_title_prompt(user_message, assistant_message);
        let result = llm_runtime
            .text_generation_call(LlmTextGenerationCallCommand {
                workspace_code: workspace_code.to_string(),
                model_code: None,
                prompt,
            })
            .await;
        let Ok(result) = result else {
            return;
        };
        let Some(title) = normalize_generated_session_title(&result.output) else {
            return;
        };
        let updated_at = current_timestamp();
        let _ = self
            .repository()
            .update_session_title(session_code, &title, &updated_at);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatSendError {
    Agent(AgentInvocationError),
    Chat(ChatError),
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_required(value: Option<String>) -> Option<String> {
    normalized_optional(value)
}

fn should_generate_manual_session_title(session: &ChatSession) -> bool {
    session.source_type == ChatSessionSourceType::Manual
        && session.task_code.is_none()
        && session
            .title
            .trim()
            .eq_ignore_ascii_case(DEFAULT_MANUAL_SESSION_TITLE)
}

fn manual_session_title_prompt(user_message: &str, assistant_message: &str) -> String {
    format!(
        "Generate a concise chat title from the first exchange.\n\
        Rules:\n\
        - Return only the title.\n\
        - Use 3 to 8 words.\n\
        - No quotes, punctuation-only decorations, markdown, or prefixes.\n\
        - Preserve product names and technical terms.\n\n\
        User message:\n{user_message}\n\n\
        Assistant response:\n{assistant_message}"
    )
}

fn normalize_generated_session_title(value: &str) -> Option<String> {
    let first_line = value.lines().map(str::trim).find(|line| !line.is_empty())?;
    let mut title = first_line
        .trim_matches(|character: char| {
            character == '"'
                || character == '\''
                || character == '`'
                || character == '*'
                || character == '#'
                || character == '-'
                || character == ':'
                || character.is_whitespace()
        })
        .trim()
        .to_string();
    for prefix in ["Title:", "title:", "Chat title:", "chat title:"] {
        if let Some(stripped) = title.strip_prefix(prefix) {
            title = stripped.trim().to_string();
            break;
        }
    }
    title = title
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() || title.eq_ignore_ascii_case(DEFAULT_MANUAL_SESSION_TITLE) {
        return None;
    }
    if title.chars().count() > MAX_GENERATED_TITLE_CHARS {
        title = title.chars().take(MAX_GENERATED_TITLE_CHARS).collect();
        title = title.trim().to_string();
    }
    (!title.is_empty()).then_some(title)
}

fn runtime_adapter_key_for_agent(agent_code: &str) -> &'static str {
    match agent_code.trim().to_ascii_lowercase().as_str() {
        "codex" => CODEX_RUNTIME_ADAPTER_KEY,
        "opencode" => OPENCODE_RUNTIME_ADAPTER_KEY,
        _ => MAIN_RUNTIME_ADAPTER_KEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manual_new_chat_session(last_run_code: Option<String>) -> ChatSession {
        ChatSession {
            id: 1,
            session_code: "SESSION-1".into(),
            workspace_code: "TRON_CORE".into(),
            app_code: None,
            task_code: None,
            source_type: ChatSessionSourceType::Manual,
            source_code: None,
            title: DEFAULT_MANUAL_SESSION_TITLE.into(),
            status: ChatSessionStatus::Active,
            agent_code: Some(MAIN_AGENT_CODE.into()),
            last_run_code,
            last_run_status: None,
            system_agent_session_id: None,
            user_code: Some("user".into()),
            created_at: "1".into(),
            updated_at: "1".into(),
        }
    }

    #[test]
    fn manual_new_chat_title_generation_retries_after_first_run() {
        let session = manual_new_chat_session(Some("RUN-1".into()));

        assert!(should_generate_manual_session_title(&session));
    }

    #[test]
    fn renamed_manual_chat_does_not_generate_title_again() {
        let mut session = manual_new_chat_session(Some("RUN-1".into()));
        session.title = "Existing title".into();

        assert!(!should_generate_manual_session_title(&session));
    }
}
