use serde_json::Value;

use crate::domain::chat::{ChatError, ChatMessageAttachment};

pub(super) fn attachments_to_storage(
    attachments: &[ChatMessageAttachment],
) -> Result<String, ChatError> {
    serde_json::to_string(attachments).map_err(|_| ChatError::StorageFailure)
}

pub(super) fn attachments_from_storage(
    value: Option<&str>,
) -> Result<Vec<ChatMessageAttachment>, ChatError> {
    let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };
    let parsed = serde_json::from_str::<Value>(raw).map_err(|_| ChatError::StorageFailure)?;
    if parsed.is_null() {
        return Ok(Vec::new());
    }
    serde_json::from_value(parsed).map_err(|_| ChatError::StorageFailure)
}
