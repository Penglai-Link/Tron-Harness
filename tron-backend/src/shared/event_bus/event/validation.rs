use std::time::{SystemTime, UNIX_EPOCH};

use crate::shared::{event_bus::config::EventBusConfig, id::IdGenerationError};

use super::{BusActorRef, BusEventAttr, BusRequestRef, BusSubjectRef};

const FORBIDDEN_FIELD_TOKENS: [&str; 13] = [
    "password",
    "secret",
    "token",
    "cookie",
    "authorization",
    "request_body",
    "response_body",
    "prompt",
    "completion",
    "plaintext",
    "oauth_code",
    "access_token",
    "refresh_token",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusEventValidationError {
    MissingField(&'static str),
    Oversize {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    ForbiddenField(&'static str),
    InvalidField(&'static str),
    IdGenerationFailed(IdGenerationError),
}

pub(super) fn bounded_required(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<String, BusEventValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BusEventValidationError::MissingField(field));
    }
    if trimmed.len() > max_bytes {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: trimmed.len(),
            limit: max_bytes,
        });
    }
    Ok(trimmed.to_string())
}

pub(super) fn bounded_optional(
    field: &'static str,
    value: Option<&str>,
    max_bytes: usize,
) -> Result<Option<String>, BusEventValidationError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > max_bytes {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: value.len(),
            limit: max_bytes,
        });
    }
    Ok(Some(value.to_string()))
}

pub(super) fn validate_subject(
    subject: BusSubjectRef,
    config: &EventBusConfig,
) -> Result<BusSubjectRef, BusEventValidationError> {
    Ok(BusSubjectRef {
        subject_type: bounded_required(
            "subject_type",
            &subject.subject_type,
            config.max_subject_type_bytes,
        )?,
        subject_code: bounded_optional(
            "subject_code",
            subject.subject_code.as_deref(),
            config.max_subject_code_bytes,
        )?,
    })
}

pub(super) fn validate_actor(
    actor: BusActorRef,
    config: &EventBusConfig,
) -> Result<BusActorRef, BusEventValidationError> {
    Ok(BusActorRef {
        user_code: bounded_required(
            "actor_user_code",
            &actor.user_code,
            config.max_actor_user_code_bytes,
        )?,
        username: bounded_required(
            "actor_username",
            &actor.username,
            config.max_actor_username_bytes,
        )?,
    })
}

pub(super) fn validate_request(
    request: BusRequestRef,
    config: &EventBusConfig,
) -> Result<BusRequestRef, BusEventValidationError> {
    Ok(BusRequestRef {
        request_id: bounded_optional(
            "request_id",
            request.request_id.as_deref(),
            config.max_request_field_bytes,
        )?,
        trace_id: bounded_optional(
            "trace_id",
            request.trace_id.as_deref(),
            config.max_request_field_bytes,
        )?,
        route: bounded_optional(
            "route",
            request.route.as_deref(),
            config.max_request_field_bytes,
        )?,
        session_code: bounded_optional(
            "session_code",
            request.session_code.as_deref(),
            config.max_request_field_bytes,
        )?,
        stream_key: bounded_optional(
            "stream_key",
            request.stream_key.as_deref(),
            config.max_request_field_bytes,
        )?,
    })
}

pub(super) fn validate_attr_list(
    field: &'static str,
    attrs: Vec<BusEventAttr>,
    max_entries: usize,
    max_key_bytes: usize,
    max_value_bytes: usize,
    max_total_bytes: Option<usize>,
) -> Result<Vec<BusEventAttr>, BusEventValidationError> {
    if attrs.len() > max_entries {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: attrs.len(),
            limit: max_entries,
        });
    }
    let mut total_bytes = 0;
    let mut validated = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let key = bounded_required("attr_key", &attr.key, max_key_bytes)?;
        ensure_safe_key(&key)?;
        let value = if field == "payload" {
            bounded_required_preserving_whitespace("attr_value", &attr.value, max_value_bytes)?
        } else {
            bounded_required("attr_value", &attr.value, max_value_bytes)?
        };
        total_bytes += key.len() + value.len();
        validated.push(BusEventAttr { key, value });
    }
    if let Some(limit) = max_total_bytes
        && total_bytes > limit
    {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: total_bytes,
            limit,
        });
    }
    Ok(validated)
}

fn ensure_safe_key(key: &str) -> Result<(), BusEventValidationError> {
    let normalized = key
        .chars()
        .map(|ch| {
            if ch == '-' {
                '_'
            } else {
                ch.to_ascii_lowercase()
            }
        })
        .collect::<String>();
    if FORBIDDEN_FIELD_TOKENS
        .iter()
        .any(|token| normalized.contains(token))
    {
        return Err(BusEventValidationError::ForbiddenField("attr_key"));
    }
    Ok(())
}

pub(super) fn bounded_required_preserving_whitespace(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<String, BusEventValidationError> {
    if value.is_empty() {
        return Err(BusEventValidationError::MissingField(field));
    }
    if value.len() > max_bytes {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: value.len(),
            limit: max_bytes,
        });
    }
    Ok(value.to_string())
}

pub(super) fn attr_bytes(attrs: &[BusEventAttr]) -> usize {
    attrs
        .iter()
        .map(|attr| attr.key.len() + attr.value.len())
        .sum()
}

pub(super) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_millis() as i64
}
