use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

use crate::domain::chat::{ChatError, ChatMessageAttachment};

const MAX_CHAT_ATTACHMENTS: usize = 5;
const MAX_CHAT_ATTACHMENT_ID_CHARS: usize = 128;
const MAX_CHAT_ATTACHMENT_KIND_CHARS: usize = 64;
const MAX_CHAT_ATTACHMENT_SUMMARY_CHARS: usize = 2_048;
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
        if attachment.kind != "inspection" {
            return Err(ChatError::AttachmentInvalid(format!(
                "unsupported attachment kind {}",
                attachment.kind
            )));
        }

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

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use serde_json::json;

    use super::*;

    #[test]
    fn normalize_attachments_redacts_nested_data_urls() {
        let data_url = format!("data:image/png;base64,{}", BASE64.encode([1_u8, 2, 3]));
        let attachment = attachment_with_image(
            None,
            json!({
                "dom": {
                    "annotationImage": {
                        "dataUrl": data_url,
                    },
                },
                "event": {
                    "payload": {
                        "annotationImage": {
                            "dataUrl": data_url,
                        },
                        "children": [
                            { "dataUrl": data_url },
                        ],
                    },
                },
            }),
        );

        let normalized = normalize_chat_attachments(vec![attachment]).expect("attachment valid");
        let normalized = normalized.first().expect("attachment retained");
        let metadata = normalized.metadata.to_string();

        assert!(
            normalized
                .image_url
                .as_deref()
                .is_some_and(|value| value.starts_with("data:image/png;base64,"))
        );
        assert!(!metadata.contains(&data_url));
        assert!(metadata.contains("[image omitted; sent as image part]"));
    }

    #[test]
    fn normalize_attachments_rejects_too_many_items() {
        let attachments = (0..=MAX_CHAT_ATTACHMENTS)
            .map(|index| attachment(format!("inspection-{index}"), None, json!({})))
            .collect::<Vec<_>>();

        assert!(matches!(
            normalize_chat_attachments(attachments),
            Err(ChatError::AttachmentInvalid(_))
        ));
    }

    #[test]
    fn normalize_attachments_rejects_invalid_base64_images() {
        let attachment = attachment_with_image(
            Some("data:image/png;base64,not-valid-base64".into()),
            json!({}),
        );

        assert!(matches!(
            normalize_chat_attachments(vec![attachment]),
            Err(ChatError::AttachmentInvalid(_))
        ));
    }

    fn attachment_with_image(image_url: Option<String>, metadata: Value) -> ChatMessageAttachment {
        attachment("inspection-1".into(), image_url, metadata)
    }

    fn attachment(id: String, image_url: Option<String>, metadata: Value) -> ChatMessageAttachment {
        ChatMessageAttachment {
            id,
            kind: "inspection".into(),
            title: None,
            summary: "comment".into(),
            image_url,
            metadata,
        }
    }
}
