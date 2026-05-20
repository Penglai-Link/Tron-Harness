use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::automation::AutomationError, response::ApiResponse};

pub fn automation_error_response(error: AutomationError) -> Response {
    let (status, code, message) = match error {
        AutomationError::NotFound => (
            StatusCode::NOT_FOUND,
            "AUTOMATION_NOT_FOUND",
            "automation was not found",
        ),
        AutomationError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        AutomationError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_WORKSPACE_INVALID",
            "automation workspace must reference an active workspace",
        ),
        AutomationError::NameRequired => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_NAME_REQUIRED",
            "automation name is required",
        ),
        AutomationError::PromptRequired => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_PROMPT_REQUIRED",
            "automation prompt is required",
        ),
        AutomationError::InvalidCron => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_CRON_INVALID",
            "automation cron expression is invalid",
        ),
        AutomationError::InvalidStatus => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_STATUS_INVALID",
            "automation status is invalid",
        ),
        AutomationError::ArchivedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "AUTOMATION_ARCHIVED_INVALID",
            "restore archived automation before modifying it",
        ),
        AutomationError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AUTOMATION_STORAGE_FAILED",
            "automation storage operation failed",
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
