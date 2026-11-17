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
    if let Some(descriptor) = crate::application::chat::composer_input::descriptor(&error) {
        let status = match error {
            ChatError::RequestedSkillForbidden => StatusCode::FORBIDDEN,
            ChatError::GoalCommandConflict => StatusCode::CONFLICT,
            ChatError::GoalCommandUnavailable => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::BAD_REQUEST,
        };
        let message = composer_error_message(&error);
        return (status, Json(ApiResponse::error(descriptor.code, message))).into_response();
    }
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
        ChatError::SessionMustBeArchived => (
            StatusCode::CONFLICT,
            "CHAT_SESSION_MUST_BE_ARCHIVED",
            "only archived chat sessions can be permanently deleted".to_string(),
        ),
        ChatError::SessionArchived => (
            StatusCode::CONFLICT,
            "CHAT_SESSION_ARCHIVED",
            "archived chat sessions cannot be changed or resumed".to_string(),
        ),
        ChatError::SessionRunActive => (
            StatusCode::CONFLICT,
            "CHAT_SESSION_RUN_ACTIVE",
            "this operation is unavailable while the chat session is running".to_string(),
        ),
        ChatError::ContainerDeleteRejected => (
            StatusCode::CONFLICT,
            "CHAT_CONTAINER_DELETE_REJECTED",
            "Disk directories are shared or unsafe to delete.".into(),
        ),
        ChatError::SessionLocationChanged => (
            StatusCode::CONFLICT,
            "CHAT_SESSION_LOCATION_CHANGED",
            "The chat location changed. Retry the request.".to_string(),
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
        ChatError::ProjectNameConflict => (
            StatusCode::CONFLICT,
            crate::application::chat::ArchiveError::TopicNameConflict
                .descriptor()
                .code,
            "An active work topic already uses this name".into(),
        ),
        ChatError::ProjectInvalid => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "CHAT_PROJECT_INVALID",
            "chat project is invalid for this workspace".to_string(),
        ),
        ChatError::DirectoryFixed => (
            StatusCode::CONFLICT,
            "CHAT_DIRECTORY_FIXED",
            "directory chats must use their own directory".to_string(),
        ),
        ChatError::DirectoryInvalid => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "CHAT_DIRECTORY_INVALID",
            "chat directory is invalid for this workspace".to_string(),
        ),
        ChatError::UserInputAlreadyAnswered => (
            StatusCode::CONFLICT,
            "CHAT_USER_INPUT_ALREADY_ANSWERED",
            "chat user input request is already answered".to_string(),
        ),
        ChatError::UserInputAnswerInvalid(message) => (
            StatusCode::BAD_REQUEST,
            "CHAT_USER_INPUT_ANSWER_INVALID",
            user_input_error_message(&message),
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
        ChatError::AnnotationsInvalid
        | ChatError::RequestedSkillsInvalid
        | ChatError::RequestedSkillForbidden
        | ChatError::GoalCommandInvalid
        | ChatError::GoalCommandUnavailable
        | ChatError::GoalCommandConflict => unreachable!("composer errors have descriptors"),
        ChatError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_STORAGE_FAILED",
            "chat storage failed".to_string(),
        ),
    };
    (
        status,
        Json(ApiResponse::error(
            code,
            super::error_messages::message(code, &message),
        )),
    )
        .into_response()
}

pub(super) fn user_input_error_message(detail: &str) -> String {
    use crate::shared::locale::SupportedLocale;
    tracing::warn!(detail, "chat interaction response rejected");
    let en = crate::shared::auth_holder::AuthHolder::get()
        .is_some_and(|auth| SupportedLocale::from_raw(&auth.locale) == SupportedLocale::EnUs);
    if en {
        "The approval or answer could not be submitted. The request may have changed or expired; refresh the conversation and try again.".into()
    } else {
        "审批或回答提交失败，请求可能已变化或过期。请刷新会话后重试。".into()
    }
}

pub(super) fn composer_error_message(error: &ChatError) -> String {
    use crate::shared::locale::SupportedLocale;
    let locale = crate::shared::auth_holder::AuthHolder::get()
        .map(|auth| SupportedLocale::from_raw(&auth.locale));
    let key = crate::application::chat::composer_input::descriptor(error)
        .expect("composer error descriptor")
        .message_key;
    let (zh, en) = match key {
        "tron.chat.requested_skill_forbidden" => (
            "所选技能未授权给当前 Agent，或已被禁用",
            "Selected skills are not granted to this Agent or have been disabled",
        ),
        "tron.chat.annotations_invalid" => (
            "评论无效，最多支持 32 条评论",
            "Invalid annotations; at most 32 comments are allowed",
        ),
        "tron.chat.requested_skills_invalid" => (
            "技能选择无效，最多选择 32 个技能",
            "Invalid skill selection; select at most 32 skills",
        ),
        "tron.chat.goal_command_invalid" => {
            ("请在 /goal 后输入目标", "Enter an objective after /goal")
        }
        "tron.chat.goal_command_conflict" => (
            "目标已发生变化，请重试",
            "The goal changed; please try again",
        ),
        "tron.chat.goal_command_unavailable" => {
            ("当前 Agent 无法使用 /goal", "This Agent cannot use /goal")
        }
        _ => unreachable!("unknown composer message key"),
    };
    if locale == Some(SupportedLocale::EnUs) {
        en.into()
    } else {
        zh.into()
    }
}

pub fn chat_send_error_response(error: ChatSendError) -> Response {
    match error {
        ChatSendError::Chat(error) => chat_error_response(error),
        ChatSendError::Agent(error) => agent_error_response(error),
        ChatSendError::RunAlreadyActive { .. } => (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "CHAT_RUN_ALREADY_ACTIVE",
                super::error_messages::message(
                    "CHAT_RUN_ALREADY_ACTIVE",
                    "chat session already has an active run",
                ),
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
    (
        status,
        Json(ApiResponse::error(
            code,
            super::error_messages::message(code, &message),
        )),
    )
        .into_response()
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
    (
        status,
        Json(ApiResponse::error(
            code,
            super::error_messages::message(code, &message),
        )),
    )
        .into_response()
}

pub fn stop_chat_run_request_rejection_response(_error: JsonRejection) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "CHAT_STOP_RUN_REQUEST_INVALID",
            super::error_messages::message(
                "CHAT_STOP_RUN_REQUEST_INVALID",
                "stop run request body is invalid",
            ),
        )),
    )
        .into_response()
}

pub fn active_chat_run_query_rejection_response(_error: QueryRejection) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "CHAT_ACTIVE_RUN_QUERY_INVALID",
            super::error_messages::message(
                "CHAT_ACTIVE_RUN_QUERY_INVALID",
                "active run query is invalid",
            ),
        )),
    )
        .into_response()
}

pub fn steer_chat_message_request_rejection_response(_error: JsonRejection) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "CHAT_STEER_REQUEST_INVALID",
            super::error_messages::message(
                "CHAT_STEER_REQUEST_INVALID",
                "steer message request body is invalid",
            ),
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
        AgentInvocationError::RuntimeFailed(message)
        | AgentInvocationError::RuntimeNotStarted(message) => {
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
    (
        status,
        Json(ApiResponse::error(
            code,
            super::error_messages::message(code, &message),
        )),
    )
        .into_response()
}
