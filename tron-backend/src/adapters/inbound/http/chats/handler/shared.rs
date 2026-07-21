use axum::response::sse::Event;
use serde_json::{Value, json};

use crate::{
    app::AppState,
    application::{
        agent::{AgentInvocationError, ResolvedAgentRuntime, build_system_runtime_model_config},
        chat::{ChatSendError, ChatSendResult, SendChatMessageCommand},
    },
    domain::{
        agent::{AgentModelConfig, AgentRuntimeType, ManagedSystemAgentRecord},
        chat::{ChatError, ChatSession},
    },
};

pub(crate) fn protocol_event_belongs_to_run(data: &Value, run_code: &str) -> bool {
    data.get("runId").and_then(Value::as_str) == Some(run_code)
}

#[derive(Debug)]
pub(crate) struct ChatSsePayload {
    pub(crate) event_name: String,
    pub(crate) data: Value,
}

impl ChatSsePayload {
    pub(crate) fn agui(event_name: impl Into<String>, data: Value) -> Self {
        Self {
            event_name: event_name.into(),
            data,
        }
    }

    pub(crate) fn event(&self) -> Event {
        Event::default()
            .event(self.event_name.clone())
            .json_data(&self.data)
            .unwrap_or_else(|_| {
                Event::default()
                    .event("RUN_ERROR")
                    .data(json!({"type":"RUN_ERROR","code":"CHAT_STREAM_SERIALIZATION_FAILED","message":"chat stream serialization failed"}).to_string())
            })
    }
}

pub(crate) enum ChatStreamCompletion {
    Success(ChatSendResult),
    Error { code: String, message: String },
}

impl ChatStreamCompletion {
    pub(crate) fn payload(&self, session_code: &str, run_code: &str) -> ChatSsePayload {
        match self {
            Self::Success(result) => ChatSsePayload::agui(
                "RUN_FINISHED",
                json!({
                    "type": "RUN_FINISHED",
                    "threadId": session_code,
                    "runId": result.run.run_code,
                    "result": result,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                }),
            ),
            Self::Error { code, message } => ChatSsePayload::agui(
                "RUN_ERROR",
                json!({
                    "type": "RUN_ERROR",
                    "threadId": session_code,
                    "runId": run_code,
                    "code": code,
                    "message": message,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                }),
            ),
        }
    }
}

pub(crate) fn chat_send_error_code(error: &ChatSendError) -> &'static str {
    match error {
        ChatSendError::Chat(ChatError::AttachmentInvalid(_)) => "CHAT_ATTACHMENT_INVALID",
        ChatSendError::Chat(ChatError::MessageRequired) => "CHAT_MESSAGE_REQUIRED",
        ChatSendError::Chat(ChatError::SessionNotFound) => "CHAT_SESSION_NOT_FOUND",
        ChatSendError::Chat(ChatError::SessionTitleRequired) => "CHAT_SESSION_TITLE_REQUIRED",
        ChatSendError::Chat(ChatError::WorkspaceRequired) => "CHAT_WORKSPACE_REQUIRED",
        ChatSendError::Chat(ChatError::WorkspaceForbidden) => "CHAT_WORKSPACE_FORBIDDEN",
        ChatSendError::Chat(ChatError::StorageFailure) => "CHAT_STORAGE_FAILED",
        ChatSendError::Agent(error) => error.code(),
        ChatSendError::RunAlreadyActive { .. } => "CHAT_RUN_ALREADY_ACTIVE",
    }
}

pub(crate) fn chat_send_error_message(error: ChatSendError) -> String {
    match error {
        ChatSendError::Chat(ChatError::AttachmentInvalid(message)) => message,
        ChatSendError::Chat(ChatError::MessageRequired) => "message content is required".into(),
        ChatSendError::Chat(ChatError::SessionNotFound) => "chat session was not found".into(),
        ChatSendError::Chat(ChatError::SessionTitleRequired) => "session title is required".into(),
        ChatSendError::Chat(ChatError::WorkspaceRequired) => "workspaceCode is required".into(),
        ChatSendError::Chat(ChatError::WorkspaceForbidden) => {
            "workspace access is forbidden".into()
        }
        ChatSendError::Chat(ChatError::StorageFailure) => "chat storage failed".into(),
        ChatSendError::Agent(error) => error.message(),
        ChatSendError::RunAlreadyActive { .. } => "chat session already has an active run".into(),
    }
}

pub(crate) fn normalized_required(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn effective_agent_code(session: &ChatSession, request: &SendChatMessageCommand) -> String {
    let selected = if session.last_run_code.is_some() {
        session.agent_code.clone()
    } else {
        normalized_optional(request.agent_code.clone()).or(session.agent_code.clone())
    };
    selected.unwrap_or_else(|| "main-agent".to_string())
}

pub(crate) fn resolve_chat_agent_runtime(
    state: &AppState,
    session: &ChatSession,
    request: &SendChatMessageCommand,
) -> Result<ResolvedAgentRuntime, ChatSendError> {
    let effective_agent_code = effective_agent_code(session, request);
    state
        .agent_management
        .resolve_workspace_agent_runtime(
            &session.workspace_code,
            Some(effective_agent_code.as_str()),
        )
        .map_err(|error| match error {
            crate::domain::agent::AgentManagementError::InvalidWorkspace => {
                ChatSendError::Chat(ChatError::WorkspaceForbidden)
            }
            crate::domain::agent::AgentManagementError::SystemAgentNotFound => {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                    "agent `{effective_agent_code}` was not found"
                )))
            }
            crate::domain::agent::AgentManagementError::SystemAgentUnavailable => {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                    "agent `{effective_agent_code}` is unavailable or disabled for this workspace"
                )))
            }
            crate::domain::agent::AgentManagementError::InternalAgentNotFound
            | crate::domain::agent::AgentManagementError::InternalMcpServerAssignmentStale(_)
            | crate::domain::agent::AgentManagementError::InternalToolAssignmentStale(_)
            | crate::domain::agent::AgentManagementError::InternalToolUnknown(_) => {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                    "agent `{effective_agent_code}` is not available for this workspace"
                )))
            }
            crate::domain::agent::AgentManagementError::StorageFailure => ChatSendError::Agent(
                AgentInvocationError::RuntimeFailed("agent runtime resolution failed".into()),
            ),
        })
}

pub(crate) fn uses_system_live_stream(runtime: &ResolvedAgentRuntime) -> bool {
    !matches!(runtime.runtime_type, AgentRuntimeType::React)
}

pub(crate) fn prepare_send_model(
    state: &AppState,
    session: &ChatSession,
    request: &SendChatMessageCommand,
    runtime: &ResolvedAgentRuntime,
) -> Result<AgentModelConfig, ChatSendError> {
    if matches!(runtime.runtime_type, AgentRuntimeType::React) {
        return state
            .models
            .text_agent_model_config(&session.workspace_code, request.model_code.as_deref())
            .map_err(|_| ChatSendError::Chat(ChatError::StorageFailure))?
            .ok_or(ChatSendError::Agent(AgentInvocationError::ModelRequired));
    }

    let model_name = request
        .system_model_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(ChatSendError::Agent(AgentInvocationError::ModelRequired))?;

    let system_agent = state
        .agent_management
        .list_workspace_system_agents(&session.workspace_code)
        .map_err(|_| {
            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "workspace system agent policy load failed".into(),
            ))
        })?
        .into_iter()
        .find(|record| record.agent_code.eq_ignore_ascii_case(&runtime.agent_code))
        .ok_or_else(|| {
            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                "agent `{}` was not found",
                runtime.agent_code
            )))
        })?;

    let matched_model = validate_system_agent_model(&system_agent, &model_name)?;
    Ok(build_system_runtime_model_config(
        &model_name,
        matched_model.and_then(|item| item.context_window_tokens),
    ))
}

fn validate_system_agent_model(
    system_agent: &ManagedSystemAgentRecord,
    model_name: &str,
) -> Result<Option<crate::domain::agent::SystemAgentModelRecord>, ChatSendError> {
    if !system_agent.enabled || !system_agent.available {
        return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
            format!(
                "agent `{}` is unavailable or disabled for this workspace",
                system_agent.agent_code
            ),
        )));
    }

    if system_agent.models.is_empty() {
        return Ok(None);
    }

    let model_matched = system_agent
        .models
        .iter()
        .find(|item| item.model_id.eq_ignore_ascii_case(model_name));
    if let Some(model) = model_matched {
        return Ok(Some(model.clone()));
    }

    Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
        format!(
            "model `{model_name}` is not available for agent `{}`",
            system_agent.agent_code
        ),
    )))
}

#[cfg(test)]
mod tests {
    use crate::application::chat::ChatSendError;

    use super::{chat_send_error_code, chat_send_error_message};

    #[test]
    fn active_run_race_uses_dedicated_stream_error_code() {
        let error = ChatSendError::RunAlreadyActive {
            active_run_code: Some("RUN-ACTIVE".into()),
        };
        assert_eq!(chat_send_error_code(&error), "CHAT_RUN_ALREADY_ACTIVE");
        assert_eq!(
            chat_send_error_message(error),
            "chat session already has an active run"
        );
    }
}
