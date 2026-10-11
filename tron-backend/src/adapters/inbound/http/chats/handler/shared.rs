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
        model_provider::{
            ModelProviderCode, ModelSelection, ModelSelectionKind, ProviderRouteCode,
        },
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
            Self::Success(result) => {
                if matches!(result.run.status, "STEP_LIMIT" | "TOOL_CALL_LIMIT") {
                    return ChatSsePayload::agui(
                        "RUN_ERROR",
                        json!({
                            "type": "RUN_ERROR", "threadId": session_code, "runId": result.run.run_code,
                            "code": result.run.status, "message": "agent execution reached its configured limit",
                            "result": result, "timestamp": chrono::Utc::now().timestamp_millis(),
                        }),
                    );
                }
                let outcome = result
                    .user_input_request
                    .as_ref()
                    .map(|request| {
                        json!({
                            "type": "interrupt",
                            "interrupts": [{
                                "id": request.request_code,
                                "reason": "input_required",
                                "message": request.intro,
                                "metadata": {
                                    "requestCode": request.request_code,
                                    "runCode": request.run_code,
                                    "questions": request.questions,
                                },
                            }],
                        })
                    })
                    .or_else(|| native_interrupt_outcome(result.run.native.as_ref()))
                    .unwrap_or_else(|| json!({ "type": "success" }));
                ChatSsePayload::agui(
                    "RUN_FINISHED",
                    json!({
                        "type": "RUN_FINISHED",
                        "threadId": session_code,
                        "runId": result.run.run_code,
                        "outcome": outcome,
                        "result": result,
                        "timestamp": chrono::Utc::now().timestamp_millis(),
                    }),
                )
            }
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
        ChatSendError::Chat(ChatError::AttachmentNotFound) => "CHAT_ATTACHMENT_NOT_FOUND",
        ChatSendError::Chat(ChatError::MessageRequired) => "CHAT_MESSAGE_REQUIRED",
        ChatSendError::Chat(ChatError::SessionNotFound) => "CHAT_SESSION_NOT_FOUND",
        ChatSendError::Chat(ChatError::SessionMustBeArchived) => "CHAT_SESSION_MUST_BE_ARCHIVED",
        ChatSendError::Chat(ChatError::SessionArchived) => "CHAT_SESSION_ARCHIVED",
        ChatSendError::Chat(ChatError::SessionRunActive) => "CHAT_SESSION_RUN_ACTIVE",
        ChatSendError::Chat(ChatError::SessionTitleRequired) => "CHAT_SESSION_TITLE_REQUIRED",
        ChatSendError::Chat(ChatError::WorkspaceRequired) => "CHAT_WORKSPACE_REQUIRED",
        ChatSendError::Chat(ChatError::WorkspaceForbidden) => "CHAT_WORKSPACE_FORBIDDEN",
        ChatSendError::Chat(ChatError::ProjectRequired) => "CHAT_PROJECT_REQUIRED",
        ChatSendError::Chat(ChatError::ProjectInvalid) => "CHAT_PROJECT_INVALID",
        ChatSendError::Chat(ChatError::UserInputAlreadyAnswered) => {
            "CHAT_USER_INPUT_ALREADY_ANSWERED"
        }
        ChatSendError::Chat(ChatError::UserInputAnswerInvalid(_)) => {
            "CHAT_USER_INPUT_ANSWER_INVALID"
        }
        ChatSendError::Chat(ChatError::UserInputPending) => "CHAT_USER_INPUT_PENDING",
        ChatSendError::Chat(ChatError::UserInputRequestNotFound) => {
            "CHAT_USER_INPUT_REQUEST_NOT_FOUND"
        }
        ChatSendError::Chat(ChatError::StorageFailure) => "CHAT_STORAGE_FAILED",
        ChatSendError::Agent(error) => error.code(),
        ChatSendError::RunAlreadyActive { .. } => "CHAT_RUN_ALREADY_ACTIVE",
    }
}

pub(crate) fn chat_send_error_message(error: ChatSendError) -> String {
    match error {
        ChatSendError::Chat(ChatError::AttachmentInvalid(message)) => message,
        ChatSendError::Chat(ChatError::AttachmentNotFound) => {
            "chat attachment was not found".into()
        }
        ChatSendError::Chat(ChatError::MessageRequired) => "message content is required".into(),
        ChatSendError::Chat(ChatError::SessionNotFound) => "chat session was not found".into(),
        ChatSendError::Chat(ChatError::SessionMustBeArchived) => {
            "only archived chat sessions can be permanently deleted".into()
        }
        ChatSendError::Chat(ChatError::SessionArchived) => {
            "archived chat sessions cannot be changed or resumed".into()
        }
        ChatSendError::Chat(ChatError::SessionRunActive) => {
            "running chat sessions cannot be archived".into()
        }
        ChatSendError::Chat(ChatError::SessionTitleRequired) => "session title is required".into(),
        ChatSendError::Chat(ChatError::WorkspaceRequired) => "workspaceCode is required".into(),
        ChatSendError::Chat(ChatError::WorkspaceForbidden) => {
            "workspace access is forbidden".into()
        }
        ChatSendError::Chat(ChatError::ProjectRequired) => {
            "projectCode is required for a manual chat session".into()
        }
        ChatSendError::Chat(ChatError::ProjectInvalid) => {
            "chat project is invalid for this workspace".into()
        }
        ChatSendError::Chat(ChatError::UserInputAlreadyAnswered) => {
            "chat user input request is already answered".into()
        }
        ChatSendError::Chat(ChatError::UserInputAnswerInvalid(message)) => message,
        ChatSendError::Chat(ChatError::UserInputPending) => {
            "chat session is waiting for a user answer".into()
        }
        ChatSendError::Chat(ChatError::UserInputRequestNotFound) => {
            "chat user input request was not found".into()
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
            | crate::domain::agent::AgentManagementError::InternalAgentCodeInvalid
            | crate::domain::agent::AgentManagementError::InternalAgentCodeConflict
            | crate::domain::agent::AgentManagementError::LoopInvalid
            | crate::domain::agent::AgentManagementError::LoopNotFound
            | crate::domain::agent::AgentManagementError::LoopConflict
            | crate::domain::agent::AgentManagementError::LoopInUse
            | crate::domain::agent::AgentManagementError::BuiltinAgentReadOnly
            | crate::domain::agent::AgentManagementError::PlatformAgentReadOnly
            | crate::domain::agent::AgentManagementError::InternalAgentGroupInvalid
            | crate::domain::agent::AgentManagementError::InternalAgentDefinitionInvalid
            | crate::domain::agent::AgentManagementError::InternalAgentPolicyInvalid
            | crate::domain::agent::AgentManagementError::ToolPermissionInvalid
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

pub(crate) fn prepare_send_model(
    state: &AppState,
    session: &ChatSession,
    request: &SendChatMessageCommand,
    runtime: &ResolvedAgentRuntime,
) -> Result<PreparedChatModel, ChatSendError> {
    if let Some(selection) = request.model_selection.as_ref() {
        validate_chat_model_selection(selection)?;
    }
    if matches!(runtime.runtime_type, AgentRuntimeType::React) {
        let is_auto = request.model_selection.as_ref().is_some_and(|selection| {
            selection.kind == crate::domain::model_provider::ModelSelectionKind::Auto
                || (selection.kind
                    == crate::domain::model_provider::ModelSelectionKind::ProviderRoute
                    && selection.route_code
                        == Some(crate::domain::model_provider::ProviderRouteCode::ChatAuto))
        });
        if is_auto {
            let router = crate::application::model_router::AgentModelRouter::new(
                state.models.clone(),
                state.model_providers.clone(),
            );
            let (model, auto_route) = router
                .resolve_initial(
                    &session.workspace_code,
                    &session.session_code,
                    &runtime.agent_code,
                    request.reasoning_effort,
                )
                .map_err(|message| {
                    ChatSendError::Agent(AgentInvocationError::RuntimeFailed(message))
                })?;
            return Ok(PreparedChatModel {
                config: model,
                auto_route: Some(auto_route),
                auto_router: Some(router),
            });
        }
        let fixed_model_code = request.model_selection.as_ref().and_then(|selection| {
            (selection.kind == crate::domain::model_provider::ModelSelectionKind::Fixed
                && selection.provider_code.is_none())
            .then(|| {
                selection
                    .model_code
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .flatten()
        });
        match state.model_providers.resolve_chat_selection(
            &session.workspace_code,
            request.model_selection.as_ref(),
            request.model_code.as_deref(),
            request.reasoning_effort,
        ) {
            Ok(Some(config)) => {
                return Ok(PreparedChatModel::fixed(config));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                    error.to_string(),
                )));
            }
        }
        let config = state
            .models
            .text_agent_model_config_with_reasoning_effort(
                &session.workspace_code,
                fixed_model_code.or(request.model_code.as_deref()),
                request.reasoning_effort,
            )
            .map_err(|error| match error {
                crate::domain::model::ModelError::ReasoningEffortUnsupported => {
                    ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
                }
                _ => ChatSendError::Chat(ChatError::StorageFailure),
            })?
            .ok_or(ChatSendError::Agent(AgentInvocationError::ModelRequired))?;
        return Ok(PreparedChatModel::fixed(config));
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
    Ok(PreparedChatModel::fixed(build_system_runtime_model_config(
        &model_name,
        matched_model.and_then(|item| item.context_window_tokens),
    )))
}

pub(crate) struct PreparedChatModel {
    pub config: AgentModelConfig,
    pub auto_route: Option<crate::domain::agent::AutoModelRouteContext>,
    pub auto_router: Option<crate::application::model_router::AgentModelRouter>,
}

impl PreparedChatModel {
    fn fixed(config: AgentModelConfig) -> Self {
        Self {
            config,
            auto_route: None,
            auto_router: None,
        }
    }
}

fn validate_chat_model_selection(selection: &ModelSelection) -> Result<(), ChatSendError> {
    let invalid = |message: &str| {
        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(message.to_owned()))
    };
    match selection.kind {
        ModelSelectionKind::Auto => {
            if selection.provider_code.is_some()
                || selection.model_code.is_some()
                || selection.quality_type.is_some()
                || selection.route_code.is_some()
            {
                return Err(invalid(
                    "Auto modelSelection cannot include a model or provider",
                ));
            }
            Ok(())
        }
        ModelSelectionKind::Fixed => {
            if selection.route_code.is_some() {
                return Err(invalid("Fixed modelSelection cannot include routeCode"));
            }
            match selection.provider_code {
                Some(ModelProviderCode::PersonAi) if selection.quality_type.is_none() => {
                    return Err(invalid(
                        "Person-AI Fixed modelSelection requires qualityType",
                    ));
                }
                None if selection.quality_type.is_some() => {
                    return Err(invalid(
                        "platform Fixed modelSelection cannot include qualityType",
                    ));
                }
                _ => {}
            }
            if selection
                .model_code
                .as_deref()
                .is_none_or(|model_code| model_code.trim().is_empty())
            {
                return Err(invalid("Fixed modelSelection requires modelCode"));
            }
            Ok(())
        }
        ModelSelectionKind::ProviderRoute => {
            if selection.provider_code != Some(ModelProviderCode::PersonAi)
                || selection.route_code != Some(ProviderRouteCode::ChatAuto)
                || selection.model_code.is_some()
                || selection.quality_type.is_some()
            {
                return Err(invalid(
                    "chat ProviderRoute modelSelection must be PersonAi ChatAuto without modelCode",
                ));
            }
            Ok(())
        }
    }
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
    use crate::domain::model_provider::{
        ModelProviderCode, ModelSelection, ModelSelectionKind, ProviderRouteCode,
    };

    use super::{chat_send_error_code, chat_send_error_message, validate_chat_model_selection};

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

    #[test]
    fn chat_provider_route_accepts_only_person_ai_auto() {
        assert!(
            validate_chat_model_selection(&ModelSelection::person_ai_route(
                ProviderRouteCode::ChatAuto,
            ))
            .is_ok()
        );
        assert!(
            validate_chat_model_selection(&ModelSelection {
                kind: ModelSelectionKind::ProviderRoute,
                provider_code: Some(ModelProviderCode::PersonAi),
                model_code: Some("deepseek-v4".into()),
                quality_type: None,
                route_code: Some(ProviderRouteCode::ChatAuto),
            })
            .is_err()
        );
    }
}

// This decorates the independently completed product response when replay failed;
// normal streaming preserves the SDK's native terminal verbatim.
fn native_interrupt_outcome(
    native: Option<&crate::domain::agent::AgentNativeRunOutcome>,
) -> Option<Value> {
    use crate::domain::agent::{AgentNativeInteractionKind, AgentNativeRunStatus};
    let native = native.filter(|native| native.status == AgentNativeRunStatus::AwaitingInput)?;
    let interrupts=native.interactions.iter().map(|interaction| {
        let message=serde_json::from_value::<eliteforge_agent_core::Content>(interaction.prompt.clone()).ok().map(|content|content.text());
        let (reason,tool_call_id)=match &interaction.kind {
            AgentNativeInteractionKind::Input=>("input_required",None),
            AgentNativeInteractionKind::Approval{invocation_id}=>("tool_call",Some(invocation_id)),
        };
        let mut value=json!({"id":interaction.id,"reason":reason,"responseSchema":interaction.response_schema,"metadata":{"nativeInteraction":interaction}});
        if let Some(message)=message {value["message"]=json!(message);}
        if let Some(id)=tool_call_id {value["toolCallId"]=json!(id);}
        value
    }).collect::<Vec<_>>();
    Some(json!({"type":"interrupt","interrupts":interrupts}))
}

#[cfg(test)]
#[path = "completion_tests.rs"]
mod completion_tests;
