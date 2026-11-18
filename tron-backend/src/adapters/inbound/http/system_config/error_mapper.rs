use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{application::system_config::SystemConfigError, response::ApiResponse};

pub fn system_config_error_response(error: SystemConfigError, locale: &str) -> Response {
    let (status, code, message_key, data) = match error {
        SystemConfigError::ReadFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SYSTEM_CONFIG_READ_FAILED",
            "system_config.read_failed",
            json!({}),
        ),
        SystemConfigError::InvalidYaml(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse {
                    code: "SYSTEM_CONFIG_INVALID",
                    message: tron_i18n::t!(
                        "system_config.invalid",
                        locale = locale,
                        detail = message
                    ),
                    data: json!({}),
                }),
            )
                .into_response();
        }
        SystemConfigError::InvalidField(path) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse {
                    code: "SYSTEM_CONFIG_FIELD_INVALID",
                    message: tron_i18n::t!(
                        "system_config.field_invalid",
                        locale = locale,
                        path = path
                    ),
                    data: json!({ "path": path }),
                }),
            )
                .into_response();
        }
        SystemConfigError::RevisionConflict => (
            StatusCode::CONFLICT,
            "SYSTEM_CONFIG_REVISION_CONFLICT",
            "system_config.revision_conflict",
            json!({}),
        ),
        SystemConfigError::MigrationConfirmationRequired(paths) => (
            StatusCode::CONFLICT,
            "SYSTEM_CONFIG_MIGRATION_CONFIRMATION_REQUIRED",
            "system_config.migration_confirmation_required",
            json!({ "paths": paths }),
        ),
        SystemConfigError::WriteFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SYSTEM_CONFIG_WRITE_FAILED",
            "system_config.write_failed",
            json!({}),
        ),
    };
    (
        status,
        Json(ApiResponse {
            code,
            message: tron_i18n::t!(message_key, locale = locale).into_owned(),
            data,
        }),
    )
        .into_response()
}
