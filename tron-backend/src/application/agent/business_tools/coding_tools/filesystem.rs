use std::{fs, path::Path};

use serde::Deserialize;

use crate::domain::agent::{AgentRunContext, Tool, ToolError, ToolRequest};

use super::{
    FsApplyPatchOutput, FsListEntry, FsListOutput, FsReadTextOutput, FsStatOutput,
    FsWriteTextOutput, apply_insert, apply_replace, ensure_parent_dirs, json_observation,
    parse_tool_input, resolve_path,
};

pub(super) struct FsReadTextTool;
pub(super) struct FsWriteTextTool;
pub(super) struct FsApplyPatchTool(
    pub std::sync::Arc<dyn crate::application::agent::CodeFileEditor>,
);
pub(super) struct FsListTool;
pub(super) struct FsStatTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FsReadTextInput {
    path: String,
    start_line: Option<usize>,
    end_line: Option<usize>,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FsWriteTextInput {
    path: String,
    content: String,
    mode: Option<WriteMode>,
    ensure_parent_dirs: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WriteMode {
    Create,
    Overwrite,
    Append,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct FsApplyPatchInput {
    path: String,
    action: PatchAction,
    target: String,
    content: Option<String>,
    replace_all: Option<bool>,
    expected_count: Option<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PatchAction {
    Replace,
    Delete,
    InsertBefore,
    InsertAfter,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FsListInput {
    path: String,
    recursive: Option<bool>,
    max_depth: Option<usize>,
    include_hidden: Option<bool>,
    files_only: Option<bool>,
    directories_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FsStatInput {
    path: String,
    follow_symlink: Option<bool>,
}

impl Tool for FsReadTextTool {
    fn name(&self) -> &str {
        "fs_read_text"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: FsReadTextInput = parse_tool_input(request.input)?;
        let path = resolve_path(context, &input.path)?;
        let content = fs::read_to_string(&path)
            .map_err(|error| ToolError::ExecutionFailed(format!("read failed: {error}")))?;
        let all_lines = content
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        let start_line = input.start_line.unwrap_or(1);
        if start_line == 0 {
            return Err(ToolError::ExecutionFailed(
                "startLine must be greater than zero".into(),
            ));
        }
        let end_line = input.end_line.unwrap_or(all_lines.len().max(start_line));
        if end_line < start_line {
            return Err(ToolError::ExecutionFailed(
                "endLine must be greater than or equal to startLine".into(),
            ));
        }
        let selected = if all_lines.is_empty() || start_line > all_lines.len() {
            String::new()
        } else {
            all_lines[start_line - 1..all_lines.len().min(end_line)].join("\n")
        };
        let max_bytes = input.max_bytes.unwrap_or(65_536);
        let bytes = selected.as_bytes();
        let truncated = bytes.len() > max_bytes;
        let rendered = if truncated {
            String::from_utf8_lossy(&bytes[..max_bytes]).to_string()
        } else {
            selected
        };
        json_observation(
            self.name(),
            &FsReadTextOutput {
                path: path.display().to_string(),
                content: rendered,
                start_line,
                end_line,
                truncated,
            },
        )
    }
}

impl Tool for FsWriteTextTool {
    fn name(&self) -> &str {
        "fs_write_text"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let _edit_guard = super::patch::EDIT_LOCK
            .lock()
            .map_err(|_| ToolError::ExecutionFailed("file editing lock unavailable".into()))?;
        let input: FsWriteTextInput = parse_tool_input(request.input)?;
        let path = resolve_path(context, &input.path)?;
        if input.ensure_parent_dirs.unwrap_or(true) {
            ensure_parent_dirs(&path)?;
        }
        let mode = input.mode.unwrap_or(WriteMode::Overwrite);
        match mode {
            WriteMode::Create => {
                if path.exists() {
                    return Err(ToolError::ExecutionFailed(
                        "target file already exists".into(),
                    ));
                }
                fs::write(&path, input.content.as_bytes())
            }
            WriteMode::Overwrite => fs::write(&path, input.content.as_bytes()),
            WriteMode::Append => append_text(&path, input.content.as_bytes()),
        }
        .map_err(|error| ToolError::ExecutionFailed(format!("write failed: {error}")))?;
        json_observation(
            self.name(),
            &FsWriteTextOutput {
                path: path.display().to_string(),
                mode: match mode {
                    WriteMode::Create => "create",
                    WriteMode::Overwrite => "overwrite",
                    WriteMode::Append => "append",
                },
                bytes_written: input.content.len(),
            },
        )
    }
}

impl Tool for FsApplyPatchTool {
    fn name(&self) -> &str {
        "fs_apply_patch"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        if request.input.get("patch").is_some() {
            return super::patch::execute(request.input, context, self.0.as_ref());
        }
        let _edit_guard = super::patch::EDIT_LOCK
            .lock()
            .map_err(|_| ToolError::ExecutionFailed("file editing lock unavailable".into()))?;
        let input: FsApplyPatchInput = parse_tool_input(request.input)?;
        if input.target.is_empty() {
            return Err(ToolError::ExecutionFailed("target is required".into()));
        }
        let path = resolve_path(context, &input.path)?;
        let mut source = fs::read_to_string(&path)
            .map_err(|error| ToolError::ExecutionFailed(format!("read failed: {error}")))?;
        let occurrences = source.matches(&input.target).count();
        let replace_all = input.replace_all.unwrap_or(false);
        if input
            .expected_count
            .is_some_and(|expected| expected != occurrences)
            || (!replace_all && occurrences > 1)
        {
            return Err(ToolError::ExecutionFailed(format!(
                "target has {occurrences} matches; provide unique context or explicitly use replaceAll with expectedCount"
            )));
        }
        let replacement_content = input.content.unwrap_or_default();
        let replacements = match input.action {
            PatchAction::Replace => apply_replace(
                &mut source,
                &input.target,
                replacement_content.as_str(),
                replace_all,
            )?,
            PatchAction::Delete => apply_replace(&mut source, &input.target, "", replace_all)?,
            PatchAction::InsertBefore => apply_insert(
                &mut source,
                &input.target,
                replacement_content.as_str(),
                replace_all,
                true,
            )?,
            PatchAction::InsertAfter => apply_insert(
                &mut source,
                &input.target,
                replacement_content.as_str(),
                replace_all,
                false,
            )?,
        };
        fs::write(&path, source.as_bytes())
            .map_err(|error| ToolError::ExecutionFailed(format!("write failed: {error}")))?;
        json_observation(
            self.name(),
            &FsApplyPatchOutput {
                path: path.display().to_string(),
                action: match input.action {
                    PatchAction::Replace => "replace",
                    PatchAction::Delete => "delete",
                    PatchAction::InsertBefore => "insert_before",
                    PatchAction::InsertAfter => "insert_after",
                },
                replacements,
            },
        )
    }
}

impl Tool for FsListTool {
    fn name(&self) -> &str {
        "fs_list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: FsListInput = parse_tool_input(request.input)?;
        let root = resolve_path(context, &input.path)?;
        let recursive = input.recursive.unwrap_or(false);
        let max_depth = input.max_depth.unwrap_or(3);
        let include_hidden = input.include_hidden.unwrap_or(false);
        let files_only = input.files_only.unwrap_or(false);
        let directories_only = input.directories_only.unwrap_or(false);
        if files_only && directories_only {
            return Err(ToolError::ExecutionFailed(
                "filesOnly and directoriesOnly cannot both be true".into(),
            ));
        }
        let mut entries = Vec::new();
        collect_entries(
            &root,
            0,
            recursive,
            max_depth,
            include_hidden,
            files_only,
            directories_only,
            &mut entries,
        )?;
        json_observation(
            self.name(),
            &FsListOutput {
                path: root.display().to_string(),
                entries,
            },
        )
    }
}

impl Tool for FsStatTool {
    fn name(&self) -> &str {
        "fs_stat"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: FsStatInput = parse_tool_input(request.input)?;
        let path = resolve_path(context, &input.path)?;
        let metadata = if input.follow_symlink.unwrap_or(false) {
            fs::metadata(&path)
        } else {
            fs::symlink_metadata(&path)
        }
        .map_err(|error| ToolError::ExecutionFailed(format!("stat failed: {error}")))?;
        json_observation(
            self.name(),
            &FsStatOutput {
                path: path.display().to_string(),
                exists: true,
                is_file: metadata.is_file(),
                is_directory: metadata.is_dir(),
                is_symlink: metadata.file_type().is_symlink(),
                size_bytes: metadata.is_file().then_some(metadata.len()),
                readonly: metadata.permissions().readonly(),
            },
        )
    }
}

fn append_text(path: &std::path::PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(bytes)
}

#[allow(clippy::too_many_arguments)]
fn collect_entries(
    path: &Path,
    depth: usize,
    recursive: bool,
    max_depth: usize,
    include_hidden: bool,
    files_only: bool,
    directories_only: bool,
    entries: &mut Vec<FsListEntry>,
) -> Result<(), ToolError> {
    let read_dir = fs::read_dir(path)
        .map_err(|error| ToolError::ExecutionFailed(format!("list failed: {error}")))?;
    for item in read_dir {
        let item = item
            .map_err(|error| ToolError::ExecutionFailed(format!("list entry failed: {error}")))?;
        let name = item.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') {
            continue;
        }
        let item_path = item.path();
        let metadata = fs::symlink_metadata(&item_path).map_err(|error| {
            ToolError::ExecutionFailed(format!("entry metadata failed: {error}"))
        })?;
        let is_file = metadata.is_file();
        let is_directory = metadata.is_dir();
        if !(files_only && !is_file || directories_only && !is_directory) {
            entries.push(FsListEntry {
                path: item_path.display().to_string(),
                name,
                is_file,
                is_directory,
                is_symlink: metadata.file_type().is_symlink(),
                size_bytes: is_file.then_some(metadata.len()),
            });
        }
        if recursive && is_directory && depth < max_depth {
            collect_entries(
                &item_path,
                depth + 1,
                recursive,
                max_depth,
                include_hidden,
                files_only,
                directories_only,
                entries,
            )?;
        }
    }
    Ok(())
}
