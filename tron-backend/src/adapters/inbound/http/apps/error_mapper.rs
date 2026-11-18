use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::app::AppError, response::ApiResponse, shared::auth_holder::AuthHolder};

pub fn app_error_response(error: AppError) -> Response {
    let locale = request_locale();
    let (status, code, message_key) = match error {
        AppError::NotFound => (StatusCode::NOT_FOUND, "APP_NOT_FOUND", "apps.not_found"),
        AppError::InvalidAppCode => (
            StatusCode::BAD_REQUEST,
            "APP_CODE_INVALID",
            "apps.code_invalid",
        ),
        AppError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "APP_WORKSPACE_REQUIRED",
            "apps.workspace_required",
        ),
        AppError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "APP_WORKSPACE_INVALID",
            "apps.workspace_invalid",
        ),
        AppError::NameRequired => (
            StatusCode::BAD_REQUEST,
            "APP_NAME_REQUIRED",
            "apps.name_required",
        ),
        AppError::LocalDirectoryRequired => (
            StatusCode::BAD_REQUEST,
            "APP_LOCAL_DIRECTORY_REQUIRED",
            "apps.local_directory_required",
        ),
        AppError::DuplicateName => (
            StatusCode::CONFLICT,
            "APP_NAME_DUPLICATE",
            "apps.name_duplicate",
        ),
        AppError::ArchivedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "APP_ARCHIVED_INVALID",
            "apps.archived_invalid",
        ),
        AppError::InUse => (StatusCode::CONFLICT, "APP_IN_USE", "apps.in_use"),
        AppError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "APP_STORAGE_FAILED",
            "apps.storage_failed",
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

fn request_locale() -> String {
    AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| tron_i18n::DEFAULT_LOCALE.as_str().to_string())
}
