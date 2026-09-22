use crate::{
    application::agent::{CodeFileChange as Change, CodeFileEditor},
    domain::agent::ToolError,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const FILE_LIMIT: usize = 8 * 1024 * 1024;
fn fail(message: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(message.to_string())
}
pub struct LocalCodeFileEditor;
impl CodeFileEditor for LocalCodeFileEditor {
    fn resolve(&self, resolved: &Path) -> Result<PathBuf, ToolError> {
        if fs::symlink_metadata(&resolved).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(fail(
                "patching symlink files is not supported; use the real file path",
            ));
        }
        let mut existing = resolved;
        let mut missing = Vec::new();
        while !existing.exists() {
            missing.push(
                existing
                    .file_name()
                    .ok_or_else(|| fail("invalid patch path"))?
                    .to_owned(),
            );
            existing = existing
                .parent()
                .ok_or_else(|| fail("path has no existing parent"))?;
        }
        let mut canonical = existing.canonicalize().map_err(fail)?;
        for part in missing.iter().rev() {
            canonical.push(part);
        }
        // A file cannot act as a parent for an added file.
        if !missing.is_empty() && !existing.is_dir() {
            return Err(fail("patch parent is not a directory"));
        }
        Ok(canonical)
    }
    fn read(&self, path: &Path) -> Result<Option<String>, ToolError> {
        read(path)
    }
    fn permissions(&self, path: &Path) -> Result<fs::Permissions, ToolError> {
        fs::metadata(path)
            .map(|meta| meta.permissions())
            .map_err(fail)
    }
    fn apply(&self, changes: &[Change]) -> Result<(), ToolError> {
        commit(changes)
    }
}
fn read(path: &Path) -> Result<Option<String>, ToolError> {
    match fs::File::open(path) {
        Ok(file) => {
            if !file.metadata().map_err(fail)?.is_file() {
                return Err(fail("patch target is not a regular file"));
            }
            let mut bytes = Vec::new();
            file.take(FILE_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(fail)?;
            if bytes.len() > FILE_LIMIT || bytes.contains(&0) {
                return Err(fail(
                    "patch supports text files up to 8 MiB; binary files are rejected",
                ));
            }
            String::from_utf8(bytes).map(Some).map_err(fail)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(fail(error)),
    }
}
fn stage(
    path: &Path,
    text: &str,
    permissions: Option<&fs::Permissions>,
) -> Result<tempfile::NamedTempFile, ToolError> {
    let parent = path
        .parent()
        .ok_or_else(|| fail("file parent unavailable"))?;
    fs::create_dir_all(parent).map_err(fail)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(fail)?;
    file.write_all(text.as_bytes()).map_err(fail)?;
    if let Some(permissions) = permissions {
        file.as_file()
            .set_permissions(permissions.clone())
            .map_err(fail)?;
    }
    Ok(file)
}
fn commit(changes: &[Change]) -> Result<(), ToolError> {
    // Stage every output and rollback copy first. No target is changed on a
    // parse, context, path, permission or staging failure.
    let mut outputs = Vec::new();
    let mut backups = Vec::new();
    for change in changes {
        outputs.push(
            change
                .after
                .as_ref()
                .map(|text| stage(&change.path, text, change.permissions.as_ref()))
                .transpose()?,
        );
        backups.push(
            change
                .before
                .as_ref()
                .map(|text| stage(&change.path, text, change.permissions.as_ref()))
                .transpose()?,
        );
    }
    for change in changes {
        if read(&change.path)? != change.before {
            return Err(fail(format!(
                "file changed during patch preparation: {}",
                change.path.display()
            )));
        }
    }
    for (index, change) in changes.iter().enumerate() {
        let result: Result<(), ToolError> = (|| {
            if read(&change.path)? != change.before {
                return Err(fail("file changed during patch application"));
            }
            if let Some(output) = outputs[index].take() {
                if change.before.is_none() {
                    output.persist_noclobber(&change.path).map_err(fail)?;
                } else {
                    output.persist(&change.path).map_err(fail)?;
                }
            } else {
                fs::remove_file(&change.path).map_err(fail)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let mut failures = Vec::new();
            for previous in (0..index).rev() {
                let old = &changes[previous];
                let rollback = (|| -> Result<(), ToolError> {
                    if read(&old.path)? != old.after {
                        return Err(fail("concurrent change; rollback skipped"));
                    }
                    if let Some(backup) = backups[previous].take() {
                        if old.after.is_none() {
                            backup.persist_noclobber(&old.path).map_err(fail)?;
                        } else {
                            backup.persist(&old.path).map_err(fail)?;
                        }
                    } else {
                        fs::remove_file(&old.path).map_err(fail)?;
                    }
                    Ok(())
                })();
                if let Err(error) = rollback {
                    failures.push(format!("{}: {error}", old.path.display()));
                }
            }
            return Err(fail(format!(
                "patch failed: {error}; {}",
                if failures.is_empty() {
                    "earlier writes rolled back".into()
                } else {
                    format!("rollback incomplete: {}", failures.join("; "))
                }
            )));
        }
    }
    Ok(())
}
