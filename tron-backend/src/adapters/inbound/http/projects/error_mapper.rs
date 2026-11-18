use crate::{
    domain::project::ProjectError, response::ApiResponse, shared::auth_holder::AuthHolder,
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub fn project_error_response(error: ProjectError) -> Response {
    let locale = request_locale();
    if error == ProjectError::DuplicateName {
        let descriptor = crate::application::project::duplicate_name_descriptor();
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::protocol_error(
                descriptor.code,
                tron_i18n::t!("tron.projects.duplicate_name", locale = &locale),
            )),
        )
            .into_response();
    }
    let (status, code, message_key) = match error {
        ProjectError::NotFound => (
            StatusCode::NOT_FOUND,
            "PROJECT_NOT_FOUND",
            "projects.not_found",
        ),
        ProjectError::ProjectCodeInvalid => (
            StatusCode::BAD_REQUEST,
            "PROJECT_CODE_INVALID",
            "projects.code_invalid",
        ),
        ProjectError::NameRequired => (
            StatusCode::BAD_REQUEST,
            "PROJECT_NAME_REQUIRED",
            "projects.name_required",
        ),
        ProjectError::DuplicateApp => (
            StatusCode::BAD_REQUEST,
            "PROJECT_APP_DUPLICATE",
            "projects.duplicate_app",
        ),
        ProjectError::DuplicateName => (
            StatusCode::CONFLICT,
            "PROJECT_NAME_DUPLICATE",
            "projects.name_duplicate",
        ),
        ProjectError::PrimaryAppInvalid => (
            StatusCode::BAD_REQUEST,
            "PROJECT_PRIMARY_APP_INVALID",
            "projects.primary_app_invalid",
        ),
        ProjectError::AppInvalid => (
            StatusCode::BAD_REQUEST,
            "PROJECT_APP_INVALID",
            "projects.app_invalid",
        ),
        ProjectError::WorkspaceInvalid => (
            StatusCode::BAD_REQUEST,
            "PROJECT_WORKSPACE_INVALID",
            "projects.workspace_invalid",
        ),
        ProjectError::Archived => (
            StatusCode::CONFLICT,
            "PROJECT_ARCHIVED",
            "projects.archived",
        ),
        ProjectError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "PROJECT_STORAGE_FAILED",
            "projects.storage_failed",
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
