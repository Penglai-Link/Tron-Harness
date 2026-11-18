use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::workspace::WorkspaceError, response::ApiResponse};

/// Locale already resolved for the current request by the profile middleware.
fn request_locale() -> String {
    crate::shared::auth_holder::AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| tron_i18n::DEFAULT_LOCALE.as_str().to_string())
}

pub fn workspace_error_response(error: WorkspaceError) -> Response {
    let locale = request_locale();
    let (status, code, message_key) = match error {
        WorkspaceError::NotFound => (
            StatusCode::NOT_FOUND,
            "ADMIN_WORKSPACE_NOT_FOUND",
            "admin.workspaces.not_found",
        ),
        WorkspaceError::InvalidWorkspaceCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_CODE_INVALID",
            "admin.workspaces.code_invalid",
        ),
        WorkspaceError::DuplicateWorkspaceCode => (
            StatusCode::CONFLICT,
            "ADMIN_WORKSPACE_CODE_DUPLICATE",
            "admin.workspaces.code_duplicate",
        ),
        WorkspaceError::WorkspaceNameRequired => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_NAME_REQUIRED",
            "admin.workspaces.name_required",
        ),
        WorkspaceError::InvalidHierarchy => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_HIERARCHY_INVALID",
            "admin.workspaces.hierarchy_invalid",
        ),
        WorkspaceError::ArchivedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_ARCHIVED_INVALID",
            "admin.workspaces.archived_invalid",
        ),
        WorkspaceError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_WORKSPACE_STORAGE_FAILED",
            "admin.workspaces.storage_failed",
        ),
    };

    (
        status,
        Json(ApiResponse::error(
            code,
            tron_i18n::t!(message_key, locale = &locale),
        )),
    )
        .into_response()
}
