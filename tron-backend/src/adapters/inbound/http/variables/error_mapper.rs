use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::variable::VariableError, response::ApiResponse};

/// Locale already resolved for the current request by the profile middleware.
fn request_locale() -> String {
    crate::shared::auth_holder::AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| tron_i18n::DEFAULT_LOCALE.as_str().to_string())
}

pub fn variable_error_response(error: VariableError) -> Response {
    let locale = request_locale();
    let (status, code, message_key) = match error {
        VariableError::NotFound => (
            StatusCode::NOT_FOUND,
            "VARIABLE_NOT_FOUND",
            "variables.not_found",
        ),
        VariableError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_WORKSPACE_REQUIRED",
            "variables.workspace_required",
        ),
        VariableError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_WORKSPACE_INVALID",
            "variables.workspace_invalid",
        ),
        VariableError::InvalidKey => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_KEY_INVALID",
            "variables.key_invalid",
        ),
        VariableError::ValueRequired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_VALUE_REQUIRED",
            "variables.value_required",
        ),
        VariableError::DuplicateKey => (
            StatusCode::CONFLICT,
            "VARIABLE_KEY_DUPLICATE",
            "variables.key_duplicate",
        ),
        VariableError::InheritedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_INHERITED_MUTATION_INVALID",
            "variables.inherited_mutation_denied",
        ),
        VariableError::EncryptionNotConfigured => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "VARIABLE_ENCRYPTION_NOT_CONFIGURED",
            "variables.encryption_not_configured",
        ),
        VariableError::EncryptFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "VARIABLE_ENCRYPT_FAILED",
            "variables.encrypt_failed",
        ),
        VariableError::DecryptFailed => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_DECRYPT_FAILED",
            "variables.decrypt_failed",
        ),
        VariableError::SensitiveUpgradeValueRequired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_SENSITIVE_UPGRADE_VALUE_REQUIRED",
            "variables.sensitive_upgrade_value_required",
        ),
        VariableError::SensitiveDowngradeValueRequired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_SENSITIVE_DOWNGRADE_VALUE_REQUIRED",
            "variables.sensitive_downgrade_value_required",
        ),
        VariableError::SensitiveOverrideRequired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_SENSITIVE_OVERRIDE_REQUIRED",
            "variables.sensitive_override_required",
        ),
        VariableError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "VARIABLE_STORAGE_FAILED",
            "variables.storage_failed",
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
