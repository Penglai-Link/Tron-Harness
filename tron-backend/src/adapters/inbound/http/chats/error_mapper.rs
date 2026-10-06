use axum::{
    Json,
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::{
        agent::AgentInvocationError,
        chat::{ChatRunControlError, ChatSendError, ChatSteerError},
    },
    domain::chat::ChatError,
    response::ApiResponse,
};

pub fn chat_error_response(error: ChatError) -> Response {
    let (status, code, message) = match error {
        ChatError::AttachmentInvalid(message) => {
            (StatusCode::BAD_REQUEST, "CHAT_ATTACHMENT_INVALID", message)
        }
        ChatError::AttachmentNotFound => (
            StatusCode::NOT_FOUND,
            "CHAT_ATTACHMENT_NOT_FOUND",
            "chat attachment was not found".to_string(),
        ),
        ChatError::MessageRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_MESSAGE_REQUIRED",
            "message content is required".to_string(),
        ),
        ChatError::SessionNotFound => (
            StatusCode::NOT_FOUND,
            "CHAT_SESSION_NOT_FOUND",
            "chat session was not found".to_string(),
        ),
        ChatError::SessionTitleRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_SESSION_TITLE_REQUIRED",
            "session title is required".to_string(),
        ),
        ChatError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_WORKSPACE_REQUIRED",
            "workspaceCode is required".to_string(),
        ),
        ChatError::WorkspaceForbidden => (
            StatusCode::FORBIDDEN,
            "CHAT_WORKSPACE_FORBIDDEN",
            "workspace access is forbidden".to_string(),
        ),
        ChatError::ProjectRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_PROJECT_REQUIRED",
            "projectCode is required for a manual chat session".to_string(),
        ),
        ChatError::ProjectInvalid => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "CHAT_PROJECT_INVALID",
            "chat project is invalid for this workspace".to_string(),
        ),
        ChatError::UserInputAlreadyAnswered => (
            StatusCode::CONFLICT,
            "CHAT_USER_INPUT_ALREADY_ANSWERED",
            "chat user input request is already answered".to_string(),
        ),
        ChatError::UserInputAnswerInvalid(message) => (
            StatusCode::BAD_REQUEST,
            "CHAT_USER_INPUT_ANSWER_INVALID",
            message,
        ),
        ChatError::UserInputPending => (
            StatusCode::CONFLICT,
            "CHAT_USER_INPUT_PENDING",
            "chat session is waiting for a user answer".to_string(),
        ),
        ChatError::UserInputRequestNotFound => (
            StatusCode::NOT_FOUND,
            "CHAT_USER_INPUT_REQUEST_NOT_FOUND",
            "chat user input request was not found".to_string(),
        ),
        ChatError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_STORAGE_FAILED",
            "chat storage failed".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn chat_send_error_response(error: ChatSendError) -> Response {
    match error {
        ChatSendError::Chat(error) => chat_error_response(error),
        ChatSendError::Agent(error) => agent_error_response(error),
        ChatSendError::RunAlreadyActive { .. } => (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "CHAT_RUN_ALREADY_ACTIVE",
                "chat session already has an active run",
            )),
        )
            .into_response(),
    }
}

pub fn chat_steer_error_response(error: ChatSteerError) -> Response {
    let (status, code, message) = match error {
        ChatSteerError::Chat(error) => return chat_error_response(error),
        ChatSteerError::SessionCodeRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_SESSION_CODE_REQUIRED",
            "sessionCode is required".to_string(),
        ),
        ChatSteerError::ExpectedRunCodeRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_EXPECTED_RUN_CODE_REQUIRED",
            "expectedRunCode is required".to_string(),
        ),
        ChatSteerError::NoActiveRun => (
            StatusCode::CONFLICT,
            "CHAT_STEER_NO_ACTIVE_RUN",
            "chat session has no active run".to_string(),
        ),
        ChatSteerError::RunMismatch { active_run_code } => (
            StatusCode::CONFLICT,
            "CHAT_STEER_RUN_MISMATCH",
            format!("expectedRunCode does not match active run `{active_run_code}`"),
        ),
        ChatSteerError::RuntimeUnsupported {
            runtime_adapter_key,
        } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "CHAT_STEER_RUNTIME_UNSUPPORTED",
            format!("runtime adapter `{runtime_adapter_key}` does not support steer"),
        ),
        ChatSteerError::RunStarting => (
            StatusCode::CONFLICT,
            "CHAT_STEER_RUN_STARTING",
            "active run is starting".to_string(),
        ),
        ChatSteerError::RunFinishing => (
            StatusCode::CONFLICT,
            "CHAT_STEER_RUN_FINISHING",
            "active run is finishing".to_string(),
        ),
        ChatSteerError::RegistryUnavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_STEER_REGISTRY_UNAVAILABLE",
            "active run registry is unavailable".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn chat_run_control_error_response(error: ChatRunControlError) -> Response {
    let (status, code, message) = match error {
        ChatRunControlError::SessionCodeRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_RUN_CONTROL_SESSION_CODE_REQUIRED",
            "sessionCode is required".to_string(),
        ),
        ChatRunControlError::ExpectedRunCodeRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_RUN_CONTROL_EXPECTED_RUN_CODE_REQUIRED",
            "expectedRunCode is required".to_string(),
        ),
        ChatRunControlError::SessionNotFound => (
            StatusCode::NOT_FOUND,
            "CHAT_RUN_CONTROL_SESSION_NOT_FOUND",
            "chat session was not found".to_string(),
        ),
        ChatRunControlError::NoActiveRun => (
            StatusCode::CONFLICT,
            "CHAT_RUN_CONTROL_NO_ACTIVE_RUN",
            "chat session has no active run".to_string(),
        ),
        ChatRunControlError::RunMismatch { active_run_code } => (
            StatusCode::CONFLICT,
            "CHAT_RUN_CONTROL_RUN_MISMATCH",
            format!("expectedRunCode does not match active run `{active_run_code}`"),
        ),
        ChatRunControlError::RunFinishing => (
            StatusCode::CONFLICT,
            "CHAT_RUN_CONTROL_RUN_FINISHING",
            "active run is finishing".to_string(),
        ),
        ChatRunControlError::RegistryUnavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_RUN_CONTROL_REGISTRY_UNAVAILABLE",
            "active run registry is unavailable".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn stop_chat_run_request_rejection_response(_error: JsonRejection) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "CHAT_STOP_RUN_REQUEST_INVALID",
            "stop run request body is invalid",
        )),
    )
        .into_response()
}

pub fn active_chat_run_query_rejection_response(_error: QueryRejection) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "CHAT_ACTIVE_RUN_QUERY_INVALID",
            "active run query is invalid",
        )),
    )
        .into_response()
}

pub fn steer_chat_message_request_rejection_response(_error: JsonRejection) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "CHAT_STEER_REQUEST_INVALID",
            "steer message request body is invalid",
        )),
    )
        .into_response()
}

fn agent_error_response(error: AgentInvocationError) -> Response {
    let (status, code, message) = match error {
        AgentInvocationError::PromptRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_PROMPT_REQUIRED",
            "prompt is required".to_string(),
        ),
        AgentInvocationError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_WORKSPACE_REQUIRED",
            "workspaceCode is required".to_string(),
        ),
        AgentInvocationError::WorkspaceInvalid => (
            StatusCode::BAD_REQUEST,
            "AGENT_WORKSPACE_INVALID",
            "workspace must reference an active workspace".to_string(),
        ),
        AgentInvocationError::ModelRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_MODEL_REQUIRED",
            "model config is required".to_string(),
        ),
        AgentInvocationError::HistoryStorageFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AGENT_HISTORY_STORAGE_FAILED",
            "agent run history storage failed".to_string(),
        ),
        AgentInvocationError::SessionBusy(_) => (
            StatusCode::CONFLICT,
            "RUN_ALREADY_ACTIVE",
            "session already has an active agent run".to_string(),
        ),
        AgentInvocationError::RunStopped => (
            StatusCode::CONFLICT,
            "AGENT_RUN_STOPPED",
            "agent run was stopped".to_string(),
        ),
        AgentInvocationError::RuntimeFailed(message) => {
            (StatusCode::BAD_REQUEST, "AGENT_RUNTIME_FAILED", message)
        }
        error @ AgentInvocationError::RuntimeVariableResolutionFailed(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error.code(),
            error.message(),
        ),
        AgentInvocationError::RuntimeSecretDeliveryUntrusted => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_RUNTIME_SECRET_DELIVERY_UNTRUSTED",
            "Sensitive runtime variable delivery is not trusted".to_string(),
        ),
        AgentInvocationError::ModelFailed(message) => {
            (StatusCode::BAD_GATEWAY, "AGENT_MODEL_FAILED", message)
        }
        AgentInvocationError::ToolFailed(message) => {
            (StatusCode::BAD_GATEWAY, "AGENT_TOOL_FAILED", message)
        }
        AgentInvocationError::EmptyFinalOutput(message) => {
            (StatusCode::BAD_GATEWAY, "AGENT_EMPTY_FINAL_OUTPUT", message)
        }
        AgentInvocationError::MaxStepsExceeded => (
            StatusCode::BAD_REQUEST,
            "AGENT_MAX_STEPS_EXCEEDED",
            "agent run exceeded max steps".to_string(),
        ),
        AgentInvocationError::MaxToolCallsExceeded => (
            StatusCode::BAD_REQUEST,
            "AGENT_MAX_TOOL_CALLS_EXCEEDED",
            "agent run exceeded max tool calls".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::application::chat::ChatRunControlError;

    use super::chat_run_control_error_response;

    #[test]
    fn run_control_errors_have_dedicated_http_statuses() {
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::SessionCodeRequired).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::ExpectedRunCodeRequired).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::SessionNotFound).status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::NoActiveRun).status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::RunMismatch {
                active_run_code: "RUN-1".into(),
            })
            .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::RunFinishing).status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            chat_run_control_error_response(ChatRunControlError::RegistryUnavailable).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
