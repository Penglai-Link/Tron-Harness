use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::task::TaskError, response::ApiResponse};

pub fn task_error_response(error: TaskError) -> Response {
    let (status, code, message) = match error {
        TaskError::NotFound => (
            StatusCode::NOT_FOUND,
            "TASK_NOT_FOUND",
            "task was not found",
        ),
        TaskError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "TASK_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        TaskError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "TASK_WORKSPACE_INVALID",
            "task workspace must reference an active workspace",
        ),
        TaskError::InvalidParent => (
            StatusCode::BAD_REQUEST,
            "TASK_PARENT_INVALID",
            "parent task must exist in the same active workspace",
        ),
        TaskError::AssigneeNotInWorkspace => (
            StatusCode::BAD_REQUEST,
            "TASK_ASSIGNEE_INVALID",
            "task assignees must be effective users in the task workspace",
        ),
        TaskError::ActiveSubtaskExists => (
            StatusCode::BAD_REQUEST,
            "TASK_ACTIVE_SUBTASK_EXISTS",
            "archive active subtasks before archiving the parent task",
        ),
        TaskError::ArchivedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "TASK_ARCHIVED_INVALID",
            "restore archived task before modifying it",
        ),
        TaskError::StatusRoleInvalid => (
            StatusCode::BAD_REQUEST,
            "TASK_STATUS_ROLE_INVALID",
            "task status does not match the inferred task role",
        ),
        TaskError::InvalidProgress | TaskError::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            "TASK_REQUEST_INVALID",
            "task request is invalid",
        ),
        TaskError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "TASK_STORAGE_FAILED",
            "task storage operation failed",
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
