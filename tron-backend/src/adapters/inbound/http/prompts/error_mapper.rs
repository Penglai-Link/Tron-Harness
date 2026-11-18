use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::domain::prompt::PromptError;

use super::response::PromptApiResponse;

/// Locale already resolved for the current request by the profile middleware.
fn request_locale() -> String {
    crate::shared::auth_holder::AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| tron_i18n::DEFAULT_LOCALE.as_str().to_string())
}

pub fn prompt_error_response(error: PromptError) -> Response {
    let locale = request_locale();
    // Dynamic validation details are produced by the prompt service and
    // forwarded as-is; the surrounding message is localized.
    let (status, code, message_key) = match error {
        PromptError::NotFound => (
            StatusCode::NOT_FOUND,
            "PROMPT_NOT_FOUND",
            "prompts.not_found",
        ),
        PromptError::InvalidInput(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(PromptApiResponse::error(
                    "PROMPT_INPUT_INVALID",
                    tron_i18n::t!(
                        "prompts.input_invalid_detail",
                        locale = &locale,
                        detail = message
                    ),
                )),
            )
                .into_response();
        }
        PromptError::DuplicateDisplayName => (
            StatusCode::CONFLICT,
            "PROMPT_DISPLAY_NAME_DUPLICATE",
            "prompts.display_name_duplicate",
        ),
        PromptError::BuiltinMutationDenied => (
            StatusCode::BAD_REQUEST,
            "PROMPT_BUILTIN_MUTATION_INVALID",
            "prompts.builtin_mutation_denied",
        ),
        PromptError::RuntimeRequired => (
            StatusCode::BAD_REQUEST,
            "PROMPT_RUNTIME_REQUIRED",
            "prompts.runtime_required",
        ),
        PromptError::VersionConflict => (
            StatusCode::CONFLICT,
            "PROMPT_VERSION_CONFLICT",
            "prompts.version_conflict",
        ),
        PromptError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "PROMPT_STORAGE_FAILED",
            "prompts.storage_failed",
        ),
    };

    (
        status,
        Json(PromptApiResponse::error(
            code,
            tron_i18n::t!(message_key, locale = &locale),
        )),
    )
        .into_response()
}
