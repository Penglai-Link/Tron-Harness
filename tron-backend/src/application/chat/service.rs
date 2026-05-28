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
    },
    domain::chat::{
        ChatError, ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession, ChatSessionGroup,
        ChatSessionStatus,
    },
    shared::time::current_timestamp,
};

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<Mutex<Box<dyn ChatRepository>>>,
}

impl ChatService {
    pub fn new(repository: impl ChatRepository + 'static) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
        }
    }

    pub fn create_session(
        &self,
        command: CreateChatSessionCommand,
        user_code: Option<String>,
    ) -> Result<ChatSession, ChatError> {
        let workspace_code =
            normalized_required(command.workspace_code).ok_or(ChatError::WorkspaceRequired)?;
        let mut repository = self.repository();
        let id = repository.next_session_id()?;
        let now = current_timestamp();
        let session = ChatSession {
            id,
            session_code: format!("SESSION-{id}"),
            workspace_code,
            user_code,
            app_code: normalized_optional(command.app_code),
            task_code: normalized_optional(command.task_code),
            title: normalized_optional(command.title).unwrap_or_else(|| "New chat".into()),
            status: ChatSessionStatus::Active,
            last_run_code: None,
            last_run_status: None,
            created_at: now.clone(),
            updated_at: now,
        };
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
            let task_title = session
                .task_code
                .clone()
                .unwrap_or_else(|| "Chat".into());
            let group = grouped
                .entry(task_title.clone())
                .or_insert_with(|| ChatSessionGroup {
                    task_code: session.task_code.clone(),
                    task_title,
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
        user_code: Option<String>,
        agent: AgentInvocationService,
    ) -> Result<ChatSendResult, ChatSendError> {
        self.send_message_inner(session_code, command, user_code, agent, None)
            .await
    }

    pub async fn send_message_stream(
        &self,
        session_code: &str,
        command: SendChatMessageCommand,
        user_code: Option<String>,
        agent: AgentInvocationService,
        sink: ModelStreamSink,
    ) -> Result<ChatSendResult, ChatSendError> {
        self.send_message_inner(session_code, command, user_code, agent, Some(sink))
            .await
    }

    async fn send_message_inner(
        &self,
        session_code: &str,
        command: SendChatMessageCommand,
        user_code: Option<String>,
        agent: AgentInvocationService,
        sink: Option<ModelStreamSink>,
    ) -> Result<ChatSendResult, ChatSendError> {
        let content = normalized_required(command.content)
            .ok_or(ChatSendError::Chat(ChatError::MessageRequired))?;
        let session = self
            .repository()
            .get_session(session_code)
            .map_err(ChatSendError::Chat)?;
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
        let user_message = self.create_message(
            session_code,
            None,
            ChatMessageRole::User,
            channel,
            "User",
            &content,
        )?;
        let invocation = AgentInvocationCommand {
            run_code: command.run_code,
            prompt: Some(content),
            workspace_code: Some(workspace_code),
            conversation_code: Some(session.session_code.clone()),
            app_code: command.app_code.or(session.app_code.clone()),
            task_code: command.task_code.or(session.task_code.clone()),
            model: command.model,
            limits: command.limits,
            tools: command.tools,
        };
        let run = if let Some(sink) = sink {
            agent.invoke_stream(invocation, user_code, sink).await
        } else {
            agent.invoke(invocation, user_code).await
        }
        .map_err(ChatSendError::Agent)?;
        let assistant_message = self.create_message(
            session_code,
            Some(run.run_code.clone()),
            ChatMessageRole::Assistant,
            channel,
            "Assistant",
            &run.final_output,
        )?;
        let updated_at = current_timestamp();
        self.repository()
            .update_session_run(session_code, &run.run_code, run.status, &updated_at)
            .map_err(ChatSendError::Chat)?;
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
    ) -> Result<ChatMessage, ChatSendError> {
        let mut repository = self.repository();
        let id = repository.next_message_id().map_err(ChatSendError::Chat)?;
        let message = ChatMessage {
            id,
            message_code: format!("MSG-{id}"),
            session_code: session_code.to_string(),
            run_code,
            role,
            channel,
            actor: actor.into(),
            content: content.into(),
            created_at: current_timestamp(),
        };
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
