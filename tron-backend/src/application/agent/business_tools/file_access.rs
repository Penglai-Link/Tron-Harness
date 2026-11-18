use std::{io::Read, path::PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};
use tron_i18n::t;

use crate::{
    application::resource::ResourceService,
    domain::agent::{AgentRunContext, ToolError},
    domain::resource::{ChatSessionAttachmentRecord, ResourceError},
};

use super::coding_tools::resolve_path;

const MAX_INPUT_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Resolve within the current session, then its server-bound delegation origin.
/// The model cannot supply the origin or widen the workspace/owner scope.
pub(super) fn read_session_attachment_range(
    resources: &ResourceService,
    context: &AgentRunContext,
    attachment_code: &str,
    start: u64,
    limit: u64,
) -> Result<(ChatSessionAttachmentRecord, u64, Vec<u8>), ResourceError> {
    let session = context
        .conversation_code
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or(ResourceError::InvalidInput)?;
    let resources = resources.for_agent_in_workspace(
        context.agent_code.as_deref(),
        context.execution_workspace_code(),
    );
    let result = resources.read_chat_session_attachment_range(
        &context.workspace_code,
        session,
        attachment_code,
        start,
        limit,
    );
    if !matches!(result, Err(ResourceError::NotFound)) {
        return result;
    }
    let origin = context
        .collaboration_owner
        .as_ref()
        .filter(|owner| {
            (owner.kind == "USER" && owner.code == context.user_code.as_deref().unwrap_or("system"))
                || owner.kind == "ASSISTANT"
        })
        .and_then(|owner| owner.origin_session_code.as_deref())
        .filter(|origin| !origin.trim().is_empty() && *origin != session);
    match origin {
        Some(origin) => resources.read_chat_session_attachment_range(
            &context.workspace_code,
            origin,
            attachment_code,
            start,
            limit,
        ),
        None => result,
    }
}

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
    resolve_input_file_bounded(input, context, resources, MAX_INPUT_FILE_BYTES)
}

pub(super) fn resolve_input_file_bounded(
    input: &Value,
    context: &AgentRunContext,
    resources: &ResourceService,
    limit: u64,
) -> Result<ResolvedInputFile, ToolError> {
    let resources = resources.for_agent_in_workspace(
        context.agent_code.as_deref(),
        context.execution_workspace_code(),
    );
    let limit = limit.min(MAX_INPUT_FILE_BYTES);
    let locale = context.locale.as_str();
    let reference = match (input.get("fileRef"), input.get("path")) {
        (Some(_), Some(_)) => {
            return Err(ToolError::ExecutionFailed(
                t!(
                    "agent_runtime.tool.file_ref.ambiguous_source",
                    locale = locale
                )
                .into_owned(),
            ));
        }
        (Some(value), None) => {
            serde_json::from_value::<AgentFileRef>(value.clone()).map_err(|e| {
                ToolError::ExecutionFailed(t!(
                    "agent_runtime.tool.file_ref.invalid_reference",
                    locale = locale,
                    detail = e
                ))
            })?
        }
        (None, Some(value)) => AgentFileRef::WorkspacePath {
            path: value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        t!("agent_runtime.tool.file_ref.path_required", locale = locale)
                            .into_owned(),
                    )
                })?
                .to_string(),
        },
        (None, None) => {
            return Err(ToolError::ExecutionFailed(
                t!("agent_runtime.tool.file_ref.required", locale = locale).into_owned(),
            ));
        }
    };

    match reference {
        AgentFileRef::WorkspacePath { path } => {
            let path = resolve_path(context, &path)?;
            if resources.storage_roots().iter().any(|root| {
                let actual_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
                path.starts_with(actual_root)
            }) {
                return Err(ToolError::ExecutionFailed(
                    t!(
                        "agent_runtime.tool.file_access.resource_storage_requires_reference",
                        locale = locale
                    )
                    .into_owned(),
                ));
            }
            let file = std::fs::File::open(&path).map_err(|e| {
                ToolError::ExecutionFailed(t!(
                    "agent_runtime.tool.file_access.read_failed",
                    locale = locale,
                    detail = e
                ))
            })?;
            let size = file
                .metadata()
                .map_err(|e| {
                    ToolError::ExecutionFailed(t!(
                        "agent_runtime.tool.file_access.metadata_failed",
                        locale = locale,
                        detail = e
                    ))
                })?
                .len();
            if size > limit {
                return Err(ToolError::ExecutionFailed(t!(
                    "agent_runtime.tool.file_access.file_too_large",
                    locale = locale,
                    limit = limit
                )));
            }
            let mut bytes = Vec::with_capacity(size as usize);
            file.take(limit.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|e| {
                    ToolError::ExecutionFailed(t!(
                        "agent_runtime.tool.file_access.read_failed",
                        locale = locale,
                        detail = e
                    ))
                })?;
            if bytes.len() as u64 > limit {
                return Err(ToolError::ExecutionFailed(t!(
                    "agent_runtime.tool.file_access.file_too_large",
                    locale = locale,
                    limit = limit
                )));
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
                    t!(
                        "agent_runtime.tool.file_ref.attachment_code_required",
                        locale = locale
                    )
                    .into_owned(),
                ));
            }
            context
                .conversation_code
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        t!(
                            "agent_runtime.tool.file_access.session_required",
                            locale = locale
                        )
                        .into_owned(),
                    )
                })?;
            let (record, _, bytes) =
                read_session_attachment_range(&resources, context, attachment_code, 0, limit)
                    .map_err(|error| {
                        ToolError::ExecutionFailed(t!(
                            "agent_runtime.tool.file_access.attachment_read_failed",
                            locale = locale,
                            attachment_code = attachment_code,
                            detail = error
                        ))
                    })?;
            if record.byte_size > limit {
                return Err(ToolError::ExecutionFailed(t!(
                    "agent_runtime.tool.file_access.attachment_too_large",
                    locale = locale,
                    limit = limit
                )));
            }
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
