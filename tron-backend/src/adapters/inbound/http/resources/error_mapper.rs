use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::domain::resource::ResourceError;

use super::ResourceHttpApiResponse;

const RESOURCE_MALFORMED_REQUEST_CODE: &str = "100100001";
const RESOURCE_NOT_FOUND_CODE: &str = "100100002";
const RESOURCE_FORBIDDEN_CODE: &str = "100100003";
const RESOURCE_CONFLICT_CODE: &str = "100100004";
const RESOURCE_INVALID_INPUT_CODE: &str = "100100005";
const RESOURCE_VARIABLE_REFERENCE_CODE: &str = "100100006";
const RESOURCE_BINARY_MISSING_CODE: &str = "100100007";
const RESOURCE_PATH_DENIED_CODE: &str = "100100008";
const RESOURCE_STORAGE_PROFILE_CODE: &str = "100100009";
const RESOURCE_STORAGE_LOCATOR_CODE: &str = "100100010";
const RESOURCE_CONNECTION_VERIFICATION_CODE: &str = "100100011";

pub fn resource_error_response(error: ResourceError, locale: &str) -> Response {
    let status = match &error {
        ResourceError::NotFound | ResourceError::MissingBinaryObject => StatusCode::NOT_FOUND,
        ResourceError::Forbidden => StatusCode::FORBIDDEN,
        ResourceError::Conflict | ResourceError::ServerHasAccounts => StatusCode::CONFLICT,
        ResourceError::ConnectionVerificationFailed(_) => StatusCode::BAD_GATEWAY,
        ResourceError::StorageFailure => StatusCode::INTERNAL_SERVER_ERROR,
        ResourceError::WorkspaceRequired
        | ResourceError::InvalidWorkspace
        | ResourceError::InvalidCode
        | ResourceError::InvalidInput
        | ResourceError::InvalidField { .. }
        | ResourceError::InvalidVariableReference
        | ResourceError::PathDenied
        | ResourceError::StorageProfileNotFound
        | ResourceError::StorageLocatorInvalid => StatusCode::BAD_REQUEST,
    };
    (
        status,
        Json(ResourceHttpApiResponse::error(
            resource_error_code(&error),
            localized_resource_error(error, locale),
        )),
    )
        .into_response()
}

pub fn resource_error_with_field(
    error: ResourceError,
    field: &str,
    _locale: &str,
) -> ResourceError {
    use crate::shared::i18n_message::MessageText;
    match error {
        ResourceError::InvalidInput => ResourceError::InvalidField {
            field: field.to_string(),
            reason: MessageText::new("resource.error.reason.invalid_input").encode(),
        },
        ResourceError::InvalidCode => ResourceError::InvalidField {
            field: field.to_string(),
            reason: MessageText::new("resource.error.reason.invalid_code").encode(),
        },
        ResourceError::InvalidField {
            field: nested,
            reason,
        } if !nested.is_empty() => {
            let field = if nested == field || nested.starts_with(&format!("{field}.")) {
                nested
            } else {
                format!("{field}.{nested}")
            };
            ResourceError::InvalidField { field, reason }
        }
        error => error,
    }
}

pub fn malformed_request_response(status: StatusCode, locale: &str) -> Response {
    (
        status,
        Json(ResourceHttpApiResponse::error(
            RESOURCE_MALFORMED_REQUEST_CODE,
            tron_i18n::t!("resource.error.malformed_request", locale = locale),
        )),
    )
        .into_response()
}

pub fn malformed_json_request_response(error: &JsonRejection, locale: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ResourceHttpApiResponse::error(
            RESOURCE_MALFORMED_REQUEST_CODE,
            localized_json_request_error(error.body_text(), locale),
        )),
    )
        .into_response()
}

fn resource_error_code(error: &ResourceError) -> &'static str {
    match error {
        ResourceError::NotFound => RESOURCE_NOT_FOUND_CODE,
        ResourceError::Forbidden => RESOURCE_FORBIDDEN_CODE,
        ResourceError::Conflict | ResourceError::ServerHasAccounts => RESOURCE_CONFLICT_CODE,
        ResourceError::MissingBinaryObject => RESOURCE_BINARY_MISSING_CODE,
        ResourceError::PathDenied => RESOURCE_PATH_DENIED_CODE,
        ResourceError::StorageProfileNotFound => RESOURCE_STORAGE_PROFILE_CODE,
        ResourceError::StorageLocatorInvalid => RESOURCE_STORAGE_LOCATOR_CODE,
        ResourceError::ConnectionVerificationFailed(_) => RESOURCE_CONNECTION_VERIFICATION_CODE,
        ResourceError::WorkspaceRequired
        | ResourceError::InvalidWorkspace
        | ResourceError::InvalidCode
        | ResourceError::InvalidInput
        | ResourceError::InvalidField { .. } => RESOURCE_INVALID_INPUT_CODE,
        ResourceError::InvalidVariableReference => RESOURCE_VARIABLE_REFERENCE_CODE,
        ResourceError::StorageFailure => "-1",
    }
}

pub fn internal_error_response(locale: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ResourceHttpApiResponse::error(
            "-1",
            tron_i18n::t!("resource.error.internal", locale = locale),
        )),
    )
        .into_response()
}

fn localized_resource_error(error: ResourceError, locale: &str) -> String {
    use tron_i18n::t;
    match error {
        ResourceError::NotFound => t!("resource.error.not_found", locale = locale).into_owned(),
        ResourceError::Forbidden => t!("resource.error.forbidden", locale = locale).into_owned(),
        ResourceError::WorkspaceRequired => {
            t!("resource.error.workspace_required", locale = locale).into_owned()
        }
        ResourceError::InvalidWorkspace => {
            t!("resource.error.invalid_workspace", locale = locale).into_owned()
        }
        ResourceError::InvalidCode => {
            t!("resource.error.invalid_code", locale = locale).into_owned()
        }
        ResourceError::InvalidInput => {
            t!("resource.error.invalid_input", locale = locale).into_owned()
        }
        ResourceError::InvalidField { field, reason } => {
            localized_invalid_field_error(field, reason, locale)
        }
        ResourceError::InvalidVariableReference => {
            t!("resource.error.invalid_variable_reference", locale = locale).into_owned()
        }
        ResourceError::Conflict => t!("resource.error.conflict", locale = locale).into_owned(),
        ResourceError::StorageFailure => {
            t!("resource.error.storage_failed", locale = locale).into_owned()
        }
        ResourceError::MissingBinaryObject => {
            t!("resource.error.binary_missing", locale = locale).into_owned()
        }
        ResourceError::PathDenied => t!("resource.error.path_denied", locale = locale).into_owned(),
        ResourceError::StorageProfileNotFound => {
            t!("resource.error.storage_profile_not_found", locale = locale).into_owned()
        }
        ResourceError::StorageLocatorInvalid => {
            t!("resource.error.storage_locator_invalid", locale = locale).into_owned()
        }
        ResourceError::ConnectionVerificationFailed(_) => t!(
            "resource.error.connection_verification_failed",
            locale = locale
        )
        .into_owned(),
        ResourceError::ServerHasAccounts => {
            t!("resource.error.server_has_accounts", locale = locale).into_owned()
        }
    }
}

fn localized_invalid_field_error(field: String, reason: String, locale: &str) -> String {
    let field = field.trim();
    // Reasons are structured messages (key + args) produced by the application
    // layer; only this display boundary turns them into text.
    let reason = match crate::shared::i18n_message::MessageText::decode(&reason) {
        Some(message) => message.render(locale),
        None => reason.trim().to_owned(),
    };
    tron_i18n::t!(
        "resource.error.invalid_field",
        locale = locale,
        field = field,
        reason = reason
    )
}

fn localized_json_request_error(detail: String, locale: &str) -> String {
    let detail = detail.trim();
    let field = [
        "missing field `",
        "missing field '",
        "unknown field `",
        "unknown field '",
        " for key `",
        " for key '",
    ]
    .iter()
    .find_map(|marker| {
        detail.find(marker).and_then(|start| {
            let value = &detail[start + marker.len()..];
            let end = value.find(['`', '\''])?;
            Some(value[..end].to_string())
        })
    });
    if let Some(field) = field {
        return tron_i18n::t!(
            "resource.error.request_field_invalid_with_field",
            locale = locale,
            field = field,
            detail = detail
        );
    }
    tron_i18n::t!(
        "resource.error.request_field_invalid",
        locale = locale,
        detail = detail
    )
}
