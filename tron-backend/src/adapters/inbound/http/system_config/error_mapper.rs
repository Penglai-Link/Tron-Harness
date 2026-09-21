use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{application::system_config::SystemConfigError, response::ApiResponse};

pub fn system_config_error_response(error: SystemConfigError) -> Response {
    let (status, code, message, data) = match error {
        SystemConfigError::ReadFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SYSTEM_CONFIG_READ_FAILED",
            "configuration file could not be read".to_owned(),
            json!({}),
        ),
        SystemConfigError::InvalidYaml(message) => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_CONFIG_INVALID",
            message,
            json!({}),
        ),
        SystemConfigError::InvalidField(path) => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_CONFIG_FIELD_INVALID",
            format!("configuration field `{path}` is not editable"),
            json!({ "path": path }),
        ),
        SystemConfigError::RevisionConflict => (
            StatusCode::CONFLICT,
            "SYSTEM_CONFIG_REVISION_CONFLICT",
            "configuration changed since it was loaded; refresh and retry".to_owned(),
            json!({}),
        ),
        SystemConfigError::MigrationConfirmationRequired(paths) => (
            StatusCode::CONFLICT,
            "SYSTEM_CONFIG_MIGRATION_CONFIRMATION_REQUIRED",
            "migration-related configuration changes require confirmation".to_owned(),
            json!({ "paths": paths }),
        ),
        SystemConfigError::WriteFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SYSTEM_CONFIG_WRITE_FAILED",
            "configuration file could not be updated atomically".to_owned(),
            json!({}),
        ),
    };
    (
        status,
        Json(ApiResponse {
            code,
            message,
            data,
        }),
    )
        .into_response()
}
