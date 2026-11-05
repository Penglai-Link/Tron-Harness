const MAX_CHAT_ATTACHMENT_BODY_OVERHEAD_BYTES: u64 = 1_048_576;
const MAX_CHAT_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const MAX_CHAT_ATTACHMENT_KIND_BYTES: usize = 64;
const MAX_CHAT_ATTACHMENT_SUMMARY_BYTES: usize = 2_048;
const MAX_CHAT_ATTACHMENT_METADATA_BYTES: usize = 256 * 1024;

struct SessionAttachmentForm {
    kind: String,
    summary: Option<String>,
    metadata: Value,
    file_name: String,
    content_type: String,
    file: std::fs::File,
}

enum SessionAttachmentFormError {
    Invalid,
    TooLarge,
    Storage,
}

pub fn chat_attachment_body_limit() -> DefaultBodyLimit {
    let maximum = MAX_CHAT_ATTACHMENT_BYTES
        .saturating_add(MAX_CHAT_ATTACHMENT_BODY_OVERHEAD_BYTES)
        .min(usize::MAX as u64) as usize;
    DefaultBodyLimit::max(maximum)
}

async fn parse_session_attachment_form(
    mut multipart: Multipart,
    maximum_bytes: u64,
) -> Result<SessionAttachmentForm, SessionAttachmentFormError> {
    let mut seen = BTreeSet::new();
    let mut kind = None;
    let mut summary = None;
    let mut metadata = None;
    let mut file = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| SessionAttachmentFormError::Invalid)?
    {
        let name = field
            .name()
            .map(str::to_owned)
            .ok_or(SessionAttachmentFormError::Invalid)?;
        if !seen.insert(name.clone()) {
            return Err(SessionAttachmentFormError::Invalid);
        }
        match name.as_str() {
            "kind" => {
                kind = Some(
                    read_session_attachment_text(field, MAX_CHAT_ATTACHMENT_KIND_BYTES).await?,
                );
            }
            "summary" => {
                summary = Some(
                    read_session_attachment_text(field, MAX_CHAT_ATTACHMENT_SUMMARY_BYTES).await?,
                );
            }
            "metadata" => {
                let value =
                    read_session_attachment_text(field, MAX_CHAT_ATTACHMENT_METADATA_BYTES).await?;
                metadata = Some(
                    serde_json::from_str(&value)
                        .map_err(|_| SessionAttachmentFormError::Invalid)?,
                );
            }
            "file" => {
                let file_name = field
                    .file_name()
                    .map(str::to_owned)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(SessionAttachmentFormError::Invalid)?;
                let content_type = field
                    .content_type()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                let mut output = tokio::fs::File::from_std(
                    tempfile::tempfile().map_err(|_| SessionAttachmentFormError::Storage)?,
                );
                let mut received = 0_u64;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|_| SessionAttachmentFormError::Invalid)?
                {
                    received = received
                        .checked_add(chunk.len() as u64)
                        .ok_or(SessionAttachmentFormError::TooLarge)?;
                    if received > maximum_bytes {
                        return Err(SessionAttachmentFormError::TooLarge);
                    }
                    tokio::io::AsyncWriteExt::write_all(&mut output, &chunk)
                        .await
                        .map_err(|_| SessionAttachmentFormError::Storage)?;
                }
                tokio::io::AsyncWriteExt::flush(&mut output)
                    .await
                    .map_err(|_| SessionAttachmentFormError::Storage)?;
                file = Some((file_name, content_type, output.into_std().await));
            }
            _ => return Err(SessionAttachmentFormError::Invalid),
        }
    }

    let (file_name, content_type, file) = file.ok_or(SessionAttachmentFormError::Invalid)?;
    let kind = kind
        .unwrap_or_else(|| {
            if content_type.starts_with("image/") {
                "image".to_string()
            } else {
                "file".to_string()
            }
        })
        .trim()
        .to_ascii_lowercase();
    if !matches!(kind.as_str(), "file" | "image" | "inspection") {
        return Err(SessionAttachmentFormError::Invalid);
    }
    let summary = summary.filter(|value| !value.trim().is_empty());
    Ok(SessionAttachmentForm {
        kind,
        summary,
        metadata: metadata.unwrap_or_else(|| json!({})),
        file_name,
        content_type,
        file,
    })
}

async fn read_session_attachment_text(
    mut field: axum::extract::multipart::Field<'_>,
    maximum: usize,
) -> Result<String, SessionAttachmentFormError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|_| SessionAttachmentFormError::Invalid)?
    {
        let length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(SessionAttachmentFormError::TooLarge)?;
        if length > maximum {
            return Err(SessionAttachmentFormError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| SessionAttachmentFormError::Invalid)
}

fn chat_attachment_form_error_response(error: SessionAttachmentFormError) -> Response {
    match error {
        SessionAttachmentFormError::Invalid => chat_error_response(ChatError::AttachmentInvalid(
            "attachment upload form is invalid".to_string(),
        )),
        SessionAttachmentFormError::TooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiResponse::error(
                "CHAT_ATTACHMENT_TOO_LARGE",
                super::error_messages::message("CHAT_ATTACHMENT_TOO_LARGE", ""),
            )),
        )
            .into_response(),
        SessionAttachmentFormError::Storage => chat_error_response(ChatError::StorageFailure),
    }
}

fn chat_attachment_request_error(status: StatusCode) -> Response {
    if status == StatusCode::PAYLOAD_TOO_LARGE {
        return chat_attachment_form_error_response(SessionAttachmentFormError::TooLarge);
    }
    chat_attachment_form_error_response(SessionAttachmentFormError::Invalid)
}

fn chat_attachment_content_disposition(
    file_name: &str,
) -> Result<HeaderValue, axum::http::header::InvalidHeaderValue> {
    let mut encoded = String::new();
    for byte in file_name.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                *byte,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            )
        {
            encoded.push(*byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}"))
}
