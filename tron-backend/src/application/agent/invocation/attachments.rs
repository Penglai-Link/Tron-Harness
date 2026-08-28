use serde_json::{Value, json};

use crate::domain::{
    agent::{AgentMessage, AgentMessagePart},
    chat::ChatMessageAttachment,
};

pub(crate) fn user_prompt_message(
    prompt: String,
    attachments: &[ChatMessageAttachment],
) -> AgentMessage {
    if attachments.is_empty() {
        return AgentMessage::user(prompt);
    }
    let mut parts = vec![AgentMessagePart::Text { text: prompt }];
    for attachment in attachments {
        parts.push(AgentMessagePart::Context {
            kind: attachment_context_kind(attachment).into(),
            text: attachment_context_text(attachment),
            metadata: Some(attachment_context_metadata(attachment)),
        });
        if let Some((mime_type, data)) = image_part_from_attachment(attachment) {
            parts.push(AgentMessagePart::Image { mime_type, data });
        }
    }
    AgentMessage::new("user", parts)
}

fn attachment_context_kind(attachment: &ChatMessageAttachment) -> &'static str {
    if attachment.kind == "inspection" {
        "page_comment"
    } else {
        "chat_attachment"
    }
}

fn attachment_context_text(attachment: &ChatMessageAttachment) -> String {
    let metadata = attachment_context_metadata(attachment);
    let tag = if attachment.kind == "inspection" {
        "page-comment"
    } else {
        "chat-attachment"
    };
    let file_details = attachment
        .file_name
        .as_deref()
        .map(|file_name| {
            let content_type = attachment
                .content_type
                .as_deref()
                .unwrap_or("application/octet-stream");
            let byte_size = attachment.byte_size.unwrap_or_default();
            format!("\nFile: {file_name} ({content_type}, {byte_size} bytes)")
        })
        .unwrap_or_default();
    let extracted_text = attachment
        .metadata
        .get("__chatAttachmentText")
        .and_then(Value::as_str)
        .map(|text| format!("\nExtracted text:\n{text}"))
        .unwrap_or_default();
    let attachment_guidance = attachment
        .resource_code
        .as_deref()
        .map(|_| {
            "\nThis is a session attachment, not a workspace path. For complete access use chat_attachment_read_text with attachmentCode for byte chunks, or chat_attachment_parse_json for full JSON parsing."
        })
        .unwrap_or_default();
    format!(
        "<{tag} id=\"{}\" kind=\"{}\">\nSummary: {}{file_details}{extracted_text}{attachment_guidance}\nMetadata: {}\n</{tag}>",
        attachment.id, attachment.kind, attachment.summary, metadata,
    )
}

fn attachment_context_metadata(attachment: &ChatMessageAttachment) -> Value {
    let mut metadata = redacted_attachment_metadata(&attachment.metadata);
    if let Some(object) = metadata.as_object_mut() {
        object.remove("__chatAttachmentText");
    }
    metadata
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

fn image_part_from_attachment(attachment: &ChatMessageAttachment) -> Option<(String, String)> {
    let data_url = attachment.image_url.as_deref().or_else(|| {
        attachment
            .metadata
            .get("dom")
            .and_then(|dom| dom.get("annotationImage"))
            .and_then(|image| image.get("dataUrl"))
            .and_then(Value::as_str)
    })?;
    let (header, data) = data_url.split_once(',')?;
    let mime_type = header
        .strip_prefix("data:")
        .and_then(|value| value.strip_suffix(";base64"))?;
    if !matches!(
        mime_type,
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        return None;
    }
    let data = data.trim();
    if data.is_empty() {
        return None;
    }
    Some((mime_type.to_string(), data.to_string()))
}
