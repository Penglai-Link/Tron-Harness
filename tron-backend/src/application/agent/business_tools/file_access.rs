use std::{io::Read, path::PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    application::resource::ResourceService,
    domain::agent::{AgentRunContext, ToolError},
};

use super::coding_tools::resolve_path;

const MAX_INPUT_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// A source-neutral reference used by document tools. The source kind controls
/// how bytes are resolved; format-specific tools only consume the resulting bytes.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum AgentFileRef {
    WorkspacePath {
        path: String,
    },
    SessionAttachment {
        #[serde(rename = "attachmentCode")]
        attachment_code: String,
    },
}

#[derive(Debug)]
pub(super) struct ResolvedInputFile {
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub workspace_path: Option<PathBuf>,
    pub reference: Value,
    pub attachment_record: Option<crate::domain::resource::ChatSessionAttachmentRecord>,
}

pub(super) fn resolve_input_file(
    input: &Value,
    context: &AgentRunContext,
    resources: &ResourceService,
) -> Result<ResolvedInputFile, ToolError> {
    let reference = match (input.get("fileRef"), input.get("path")) {
        (Some(_), Some(_)) => {
            return Err(ToolError::ExecutionFailed(
                "provide fileRef or legacy path, not both".into(),
            ));
        }
        (Some(value), None) => serde_json::from_value::<AgentFileRef>(value.clone())
            .map_err(|e| ToolError::ExecutionFailed(format!("fileRef is invalid: {e}")))?,
        (None, Some(value)) => AgentFileRef::WorkspacePath {
            path: value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| ToolError::ExecutionFailed("path is required".into()))?
                .to_string(),
        },
        (None, None) => {
            return Err(ToolError::ExecutionFailed(
                "fileRef is required (legacy path is also accepted)".into(),
            ));
        }
    };

    match reference {
        AgentFileRef::WorkspacePath { path } => {
            let path = resolve_path(context, &path)?;
            let file = std::fs::File::open(&path)
                .map_err(|e| ToolError::ExecutionFailed(format!("file read failed: {e}")))?;
            let size = file
                .metadata()
                .map_err(|e| ToolError::ExecutionFailed(format!("file metadata failed: {e}")))?
                .len();
            if size > MAX_INPUT_FILE_BYTES {
                return Err(ToolError::ExecutionFailed(
                    "file exceeds 32 MiB limit".into(),
                ));
            }
            let mut bytes = Vec::with_capacity(size as usize);
            file.take(MAX_INPUT_FILE_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| ToolError::ExecutionFailed(format!("file read failed: {e}")))?;
            if bytes.len() as u64 > MAX_INPUT_FILE_BYTES {
                return Err(ToolError::ExecutionFailed(
                    "file exceeds 32 MiB limit".into(),
                ));
            }
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned();
            Ok(ResolvedInputFile {
                file_name,
                bytes,
                workspace_path: Some(path.clone()),
                reference: json!({"kind":"workspace_path","path":path}),
                attachment_record: None,
            })
        }
        AgentFileRef::SessionAttachment { attachment_code } => {
            let attachment_code = attachment_code.trim();
            if attachment_code.is_empty() {
                return Err(ToolError::ExecutionFailed(
                    "fileRef.attachmentCode is required".into(),
                ));
            }
            let session_code = context
                .conversation_code
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "session attachment fileRef requires a conversation session".into(),
                    )
                })?;
            let (record, bytes) = resources
                .read_chat_session_attachment(
                    &context.workspace_code,
                    session_code,
                    attachment_code,
                )
                .map_err(|error| {
                    ToolError::ExecutionFailed(format!(
                        "session attachment `{attachment_code}` could not be read: {error}"
                    ))
                })?;
            Ok(ResolvedInputFile {
                file_name: record.file_name.clone(),
                bytes,
                workspace_path: None,
                reference: json!({
                    "kind":"session_attachment",
                    "attachmentCode":record.attachment_code,
                }),
                attachment_record: Some(record),
            })
        }
    }
}
