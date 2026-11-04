use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

use crate::domain::chat::{ChatError, ChatMessageAttachment};

const MAX_CHAT_ATTACHMENTS: usize = 5;
const MAX_CHAT_ATTACHMENT_ID_CHARS: usize = 128;
const MAX_CHAT_ATTACHMENT_KIND_CHARS: usize = 64;
const MAX_CHAT_ATTACHMENT_SUMMARY_CHARS: usize = 2_048;
const MAX_CHAT_ATTACHMENT_FILE_NAME_CHARS: usize = 255;
const MAX_CHAT_ATTACHMENT_CONTENT_TYPE_CHARS: usize = 255;
const MAX_CHAT_ATTACHMENT_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_CHAT_ATTACHMENT_TOTAL_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_CHAT_ATTACHMENT_METADATA_BYTES: usize = 256 * 1024;

pub(super) fn normalize_chat_attachments(
    attachments: Vec<ChatMessageAttachment>,
) -> Result<Vec<ChatMessageAttachment>, ChatError> {
    if attachments.len() > MAX_CHAT_ATTACHMENTS {
        return Err(ChatError::AttachmentInvalid(format!(
            "at most {MAX_CHAT_ATTACHMENTS} attachments are supported"
        )));
    }

    let mut total_image_bytes = 0usize;
    let mut normalized = Vec::with_capacity(attachments.len());
    for mut attachment in attachments {
        validate_attachment_text(
            "attachment id",
            &attachment.id,
            MAX_CHAT_ATTACHMENT_ID_CHARS,
        )?;
        validate_attachment_text(
            "attachment kind",
            &attachment.kind,
            MAX_CHAT_ATTACHMENT_KIND_CHARS,
        )?;
        validate_attachment_text(
            "attachment summary",
            &attachment.summary,
            MAX_CHAT_ATTACHMENT_SUMMARY_CHARS,
        )?;
        let kind = attachment.kind.trim().to_ascii_lowercase();
        if !matches!(kind.as_str(), "inspection" | "file" | "image") {
            return Err(ChatError::AttachmentInvalid(format!(
                "unsupported attachment kind {}",
                attachment.kind
            )));
        }
        attachment.kind = kind;

        if let Some(title) = attachment.title.as_deref() {
            validate_attachment_text("attachment title", title, MAX_CHAT_ATTACHMENT_SUMMARY_CHARS)?;
        }
        if let Some(file_name) = attachment.file_name.as_deref() {
            validate_attachment_text(
                "attachment file name",
                file_name,
                MAX_CHAT_ATTACHMENT_FILE_NAME_CHARS,
            )?;
        }
        if let Some(content_type) = attachment.content_type.as_deref() {
            validate_attachment_text(
                "attachment content type",
                content_type,
                MAX_CHAT_ATTACHMENT_CONTENT_TYPE_CHARS,
            )?;
        }

        let resource_backed = attachment.resource_code.is_some();
        if let Some(resource_code) = attachment.resource_code.as_deref() {
            validate_attachment_resource_code(resource_code)?;
        }
        if resource_backed && attachment.image_url.is_some() {
            return Err(ChatError::AttachmentInvalid(
                "resource-backed attachments cannot include inline image data".into(),
            ));
        }
        if !resource_backed && attachment.kind != "inspection" {
            return Err(ChatError::AttachmentInvalid(
                "file and image attachments must reference a session attachment".into(),
            ));
        }

        if resource_backed {
            attachment.image_url = None;
        } else {
            let image_url = attachment
                .image_url
                .as_deref()
                .or_else(|| attachment_image_data_url_from_metadata(&attachment.metadata));
            attachment.image_url = match image_url {
                Some(value) => {
                    let (data_url, byte_len) = validate_attachment_image_data_url(value)?;
                    total_image_bytes = total_image_bytes.saturating_add(byte_len);
                    if total_image_bytes > MAX_CHAT_ATTACHMENT_TOTAL_IMAGE_BYTES {
                        return Err(ChatError::AttachmentInvalid(format!(
                            "attachment images exceed {} bytes in total",
                            MAX_CHAT_ATTACHMENT_TOTAL_IMAGE_BYTES
                        )));
                    }
                    Some(data_url)
                }
                None => None,
            };
        }

        let metadata = redacted_attachment_metadata(&attachment.metadata);
        let metadata_size = serde_json::to_vec(&metadata)
            .map_err(|_| ChatError::AttachmentInvalid("attachment metadata is invalid".into()))?
            .len();
        if metadata_size > MAX_CHAT_ATTACHMENT_METADATA_BYTES {
            return Err(ChatError::AttachmentInvalid(format!(
                "attachment metadata exceeds {} bytes",
                MAX_CHAT_ATTACHMENT_METADATA_BYTES
            )));
        }
        attachment.metadata = metadata;
        normalized.push(attachment);
    }
    Ok(normalized)
}

fn validate_attachment_resource_code(value: &str) -> Result<(), ChatError> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > MAX_CHAT_ATTACHMENT_ID_CHARS
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(ChatError::AttachmentInvalid(
            "attachment resourceCode is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_attachment_text(label: &str, value: &str, max_chars: usize) -> Result<(), ChatError> {
    if value.trim().is_empty() {
        return Err(ChatError::AttachmentInvalid(format!("{label} is required")));
    }
    if value.chars().count() > max_chars {
        return Err(ChatError::AttachmentInvalid(format!(
            "{label} exceeds {max_chars} characters"
        )));
    }
    Ok(())
}

fn validate_attachment_image_data_url(value: &str) -> Result<(String, usize), ChatError> {
    let value = value.trim();
    let (header, data) = value.split_once(',').ok_or_else(|| {
        ChatError::AttachmentInvalid("attachment image must be a data URL".into())
    })?;
    let mime_type = header
        .strip_prefix("data:")
        .and_then(|value| value.strip_suffix(";base64"))
        .ok_or_else(|| {
            ChatError::AttachmentInvalid("attachment image must be base64 encoded".into())
        })?;
    if !matches!(
        mime_type,
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        return Err(ChatError::AttachmentInvalid(format!(
            "unsupported attachment image type {mime_type}"
        )));
    }
    let data = data.trim();
    let decoded = BASE64
        .decode(data.as_bytes())
        .map_err(|_| ChatError::AttachmentInvalid("attachment image base64 is invalid".into()))?;
    if decoded.is_empty() {
        return Err(ChatError::AttachmentInvalid(
            "attachment image cannot be empty".into(),
        ));
    }
    if decoded.len() > MAX_CHAT_ATTACHMENT_IMAGE_BYTES {
        return Err(ChatError::AttachmentInvalid(format!(
            "attachment image exceeds {} bytes",
            MAX_CHAT_ATTACHMENT_IMAGE_BYTES
        )));
    }
    Ok((format!("data:{mime_type};base64,{data}"), decoded.len()))
}

fn attachment_image_data_url_from_metadata(metadata: &Value) -> Option<&str> {
    metadata
        .get("dom")
        .and_then(|dom| dom.get("annotationImage"))
        .and_then(|image| image.get("dataUrl"))
        .and_then(Value::as_str)
}

fn redacted_attachment_metadata(metadata: &Value) -> Value {
    let mut value = metadata.clone();
    redact_attachment_metadata_value(&mut value);
    value
}

fn redact_attachment_metadata_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if key == "dataUrl"
                    && value
                        .as_str()
                        .is_some_and(|text| text.starts_with("data:image/"))
                {
                    *value = json!("[image omitted; sent as image part]");
                } else {
                    redact_attachment_metadata_value(value);
                }
            }
        }
        Value::Array(items) => {
            items.iter_mut().for_each(redact_attachment_metadata_value);
        }
        _ => {}
    }
}
