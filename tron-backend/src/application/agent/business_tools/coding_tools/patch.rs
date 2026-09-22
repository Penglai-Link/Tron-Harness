//! Multi-file editing: validate the whole patch before writing any file.
use super::{json_observation, resolve_optional_directory, resolve_path};
use crate::application::agent::{CodeFileChange as Change, CodeFileEditor};
use crate::domain::agent::{
    AgentRunContext, FilePatch, ToolError, ToolExecutionResult, apply_text_hunks, parse_text_patch,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

pub(super) static EDIT_LOCK: Mutex<()> = Mutex::new(());
const FILE_LIMIT: usize = 8 * 1024 * 1024;
const TOTAL_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PatchInput {
    patch: String,
    cwd: Option<String>,
    #[serde(default)]
    dry_run: bool,
}
fn fail(message: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(message.to_string())
}

fn checked_path(
    editor: &dyn CodeFileEditor,
    context: &AgentRunContext,
    cwd: &Path,
    raw: &str,
) -> Result<PathBuf, ToolError> {
    if raw.trim().is_empty()
        || Path::new(raw)
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(fail("patch paths must be nonempty and must not contain .."));
    }
    let absolute = cwd.join(raw);
    let resolved = resolve_path(
        context,
        absolute
            .to_str()
            .ok_or_else(|| fail("invalid path encoding"))?,
    )?;
    editor.resolve(&resolved)
}

fn push(
    changes: &mut BTreeMap<PathBuf, Change>,
    path: PathBuf,
    before: Option<String>,
    after: Option<String>,
    permissions: Option<fs::Permissions>,
) -> Result<(), ToolError> {
    if changes
        .keys()
        .any(|existing| existing.starts_with(&path) || path.starts_with(existing))
    {
        return Err(fail(format!(
            "duplicate or overlapping file operation: {}",
            path.display()
        )));
    }
    if after
        .as_ref()
        .is_some_and(|text| text.len() > FILE_LIMIT || text.contains('\0'))
    {
        return Err(fail("patched file exceeds text file limits"));
    }
    changes.insert(
        path.clone(),
        Change {
            path,
            before,
            after,
            permissions,
        },
    );
    Ok(())
}

pub(super) fn execute(
    input: serde_json::Value,
    context: &AgentRunContext,
    editor: &dyn CodeFileEditor,
) -> Result<ToolExecutionResult, ToolError> {
    let input: PatchInput = serde_json::from_value(input).map_err(fail)?;
    let operations = parse_text_patch(&input.patch).map_err(fail)?;
    let _guard = EDIT_LOCK
        .lock()
        .map_err(|_| fail("file editing lock unavailable"))?;
    let cwd = resolve_optional_directory(context, input.cwd.as_deref())?
        .ok_or_else(|| fail("working directory unavailable"))?
        .canonicalize()
        .map_err(fail)?;
    let mut changes = BTreeMap::new();
    for operation in operations {
        match operation {
            FilePatch::Add { path, content } => {
                let path = checked_path(editor, context, &cwd, &path)?;
                if editor.read(&path)?.is_some() {
                    return Err(fail(format!("file already exists: {}", path.display())));
                }
                push(&mut changes, path, None, Some(content), None)?;
            }
            FilePatch::Delete { path } => {
                let path = checked_path(editor, context, &cwd, &path)?;
                let before = editor
                    .read(&path)?
                    .ok_or_else(|| fail(format!("file not found: {}", path.display())))?;
                let permissions = editor.permissions(&path)?;
                push(&mut changes, path, Some(before), None, Some(permissions))?;
            }
            FilePatch::Update {
                path,
                destination,
                hunks,
            } => {
                let path = checked_path(editor, context, &cwd, &path)?;
                let before = editor
                    .read(&path)?
                    .ok_or_else(|| fail(format!("file not found: {}", path.display())))?;
                let after = apply_text_hunks(&before, &hunks)
                    .map_err(|error| fail(format!("{}: {error}", path.display())))?;
                let permissions = editor.permissions(&path)?;
                if let Some(destination) = destination {
                    let destination = checked_path(editor, context, &cwd, &destination)?;
                    if destination == path || editor.read(&destination)?.is_some() {
                        return Err(fail(
                            "move destination must be a different, nonexistent file",
                        ));
                    }
                    push(
                        &mut changes,
                        path,
                        Some(before),
                        None,
                        Some(permissions.clone()),
                    )?;
                    push(
                        &mut changes,
                        destination,
                        None,
                        Some(after),
                        Some(permissions),
                    )?;
                } else {
                    push(
                        &mut changes,
                        path,
                        Some(before),
                        Some(after),
                        Some(permissions),
                    )?;
                }
            }
        }
        let total: usize = changes
            .values()
            .map(|change| {
                change.before.as_ref().map_or(0, String::len)
                    + change.after.as_ref().map_or(0, String::len)
            })
            .sum();
        if total > TOTAL_LIMIT {
            return Err(fail("patch working set exceeds 32 MiB"));
        }
    }
    let changes: Vec<_> = changes.into_values().collect();
    let mut diff = String::new();
    let mut truncated = false;
    for change in &changes {
        let rendered = preview_diff(change);
        let available = 65536usize.saturating_sub(diff.len());
        if rendered.len() > available {
            let mut end = available;
            while !rendered.is_char_boundary(end) {
                end -= 1;
            }
            diff.push_str(&rendered[..end]);
            truncated = true;
        } else {
            diff.push_str(&rendered);
        }
    }
    if !input.dry_run {
        editor.apply(&changes)?;
    }
    json_observation(
        "fs_apply_patch",
        &json!({
            "cwd":cwd,"applied":!input.dry_run,"dryRun":input.dry_run,"diff":diff,"diffTruncated":truncated,
            "files":changes.iter().map(|change|json!({
                "path":change.path,"action":if change.before.is_none(){"add"}else if change.after.is_none(){"delete"}else{"update"}
            })).collect::<Vec<_>>()
        }),
    )
}

fn preview_diff(change: &Change) -> String {
    let before: Vec<_> = change
        .before
        .as_deref()
        .unwrap_or_default()
        .lines()
        .collect();
    let after: Vec<_> = change
        .after
        .as_deref()
        .unwrap_or_default()
        .lines()
        .collect();
    let prefix = before
        .iter()
        .zip(&after)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = before[prefix..]
        .iter()
        .rev()
        .zip(after[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let start = prefix.saturating_sub(3);
    let old_end = (before.len() - suffix + 3).min(before.len());
    let new_end = (after.len() - suffix + 3).min(after.len());
    let mut diff = format!(
        "--- {}\n+++ {}\n@@ -{},{} +{},{} @@\n",
        if change.before.is_some() {
            change.path.display().to_string()
        } else {
            "/dev/null".into()
        },
        if change.after.is_some() {
            change.path.display().to_string()
        } else {
            "/dev/null".into()
        },
        if before.is_empty() { 0 } else { start + 1 },
        old_end - start,
        if after.is_empty() { 0 } else { start + 1 },
        new_end - start
    );
    for line in &before[start..prefix] {
        diff.push_str(&format!(" {line}\n"));
    }
    for line in &before[prefix..before.len() - suffix] {
        diff.push_str(&format!("-{line}\n"));
    }
    for line in &after[prefix..after.len() - suffix] {
        diff.push_str(&format!("+{line}\n"));
    }
    for line in &before[before.len() - suffix..old_end] {
        diff.push_str(&format!(" {line}\n"));
    }
    diff
}
