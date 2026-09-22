use crate::application::project::ProjectInstructionFile;
use std::{fs, io::Read, path::Path};

const LIMIT: usize = 32 * 1024;
/// Project-only instructions. Never inspect machine-wide agent credentials/settings,
/// follow external imports, or traverse sibling worktrees.
pub(super) fn read(directory: &str) -> Result<Vec<ProjectInstructionFile>, String> {
    let cwd = Path::new(directory)
        .canonicalize()
        .map_err(|e| format!("project instruction directory unavailable: {e}"))?;
    if !cwd.is_dir() {
        return Err("project instruction cwd is not a directory".into());
    }
    let root = cwd
        .ancestors()
        .find(|path| path.join(".git").exists())
        .unwrap_or(&cwd);
    let mut directories: Vec<_> = cwd
        .ancestors()
        .take_while(|path| path.starts_with(root))
        .collect();
    directories.reverse();
    let mut remaining = LIMIT;
    let mut files = Vec::new();
    for directory in directories {
        // One instruction source per directory; fallback avoids duplicate or
        // contradictory instructions in repositories supporting several agents.
        for name in [
            "AGENTS.override.md",
            "AGENTS.md",
            "CLAUDE.md",
            ".claude/CLAUDE.md",
        ] {
            let path = directory.join(name);
            let canonical = match path.canonicalize() {
                Ok(value) => value,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    return Err(format!(
                        "cannot resolve project instructions {}: {e}",
                        path.display()
                    ));
                }
            };
            if !canonical.starts_with(root) {
                return Err(format!(
                    "project instruction file escapes current repository: {}",
                    path.display()
                ));
            }
            if !fs::metadata(&canonical)
                .map_err(|e| e.to_string())?
                .is_file()
            {
                return Err(format!(
                    "project instruction path is not a regular file: {}",
                    path.display()
                ));
            }
            let mut file = fs::File::open(&canonical)
                .map_err(|e| format!("cannot read project instructions {}: {e}", path.display()))?;
            if !file.metadata().map_err(|e| e.to_string())?.is_file() {
                return Err(format!(
                    "project instruction path is not a regular file: {}",
                    path.display()
                ));
            }
            let mut bytes = Vec::new();
            (&mut file)
                .take((remaining + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            let truncated = bytes.len() > remaining;
            bytes.truncate(remaining);
            if truncated {
                while std::str::from_utf8(&bytes).is_err()
                    && bytes.last().is_some_and(|byte| *byte >= 128)
                {
                    bytes.pop();
                }
            }
            let content = String::from_utf8(bytes)
                .map_err(|_| format!("project instructions must be UTF-8: {}", path.display()))?;
            if content.trim().is_empty() && !truncated {
                continue;
            }
            remaining = remaining.saturating_sub(content.len());
            files.push(ProjectInstructionFile {
                path: path.to_string_lossy().into_owned(),
                content,
                truncated,
            });
            if truncated || remaining == 0 {
                return Ok(files);
            }
            break;
        }
    }
    Ok(files)
}
