use std::{fs, path::Path, process::Command};

use serde::Deserialize;

use crate::domain::agent::{AgentRunContext, Tool, ToolError, ToolRequest};

use super::{
    SearchGlobOutput, SearchGrepOutput, SearchPathEntry, SearchPathOutput, default_cwd,
    json_observation, parse_tool_input, resolve_optional_directory,
};

pub(super) struct SearchGrepTool;
pub(super) struct SearchGlobTool;
pub(super) struct SearchPathTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchGrepInput {
    pattern: String,
    root: Option<String>,
    #[serde(default)]
    glob: Vec<String>,
    case_sensitive: Option<bool>,
    word: Option<bool>,
    context_lines: Option<usize>,
    max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchGlobInput {
    patterns: Vec<String>,
    root: Option<String>,
    include_hidden: Option<bool>,
    max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPathInput {
    patterns: Vec<String>,
    root: Option<String>,
    entry_type: Option<SearchPathEntryType>,
    include_hidden: Option<bool>,
    max_depth: Option<usize>,
    max_results: Option<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SearchPathEntryType {
    Any,
    File,
    Directory,
}

impl Tool for SearchGrepTool {
    fn name(&self) -> &str {
        "search.grep"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: SearchGrepInput = parse_tool_input(request.input)?;
        let pattern = input.pattern.trim();
        if pattern.is_empty() {
            return Err(ToolError::ExecutionFailed("pattern is required".into()));
        }
        let root =
            resolve_optional_directory(context, input.root.as_deref())?.unwrap_or_else(default_cwd);
        let mut command = Command::new("rg");
        command.arg("--no-heading").arg("-n");
        if !input.case_sensitive.unwrap_or(false) {
            command.arg("-i");
        }
        if input.word.unwrap_or(false) {
            command.arg("-w");
        }
        if let Some(context_lines) = input.context_lines
            && context_lines > 0
        {
            command.arg("-C").arg(context_lines.to_string());
        }
        command
            .arg("-m")
            .arg(input.max_results.unwrap_or(200).to_string());
        for glob in input.glob {
            command.arg("-g").arg(glob);
        }
        command.arg(pattern).arg(".");
        command.current_dir(&root);
        let output = command
            .output()
            .map_err(|error| ToolError::ExecutionFailed(format!("rg execution failed: {error}")))?;
        if !output.status.success() && output.status.code() != Some(1) {
            return Err(ToolError::ExecutionFailed(format!(
                "rg exited with status {}: {}",
                output
                    .status
                    .code()
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        json_observation(
            self.name(),
            &SearchGrepOutput {
                root: root.display().to_string(),
                pattern: pattern.to_string(),
                lines,
            },
        )
    }
}

impl Tool for SearchGlobTool {
    fn name(&self) -> &str {
        "search.glob"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: SearchGlobInput = parse_tool_input(request.input)?;
        if input.patterns.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "patterns must contain at least one value".into(),
            ));
        }
        let root =
            resolve_optional_directory(context, input.root.as_deref())?.unwrap_or_else(default_cwd);
        let mut command = Command::new("rg");
        command.arg("--files");
        if input.include_hidden.unwrap_or(false) {
            command.arg("--hidden");
        }
        for pattern in input.patterns.iter() {
            command.arg("-g").arg(pattern);
        }
        command.current_dir(&root);
        let output = command
            .output()
            .map_err(|error| ToolError::ExecutionFailed(format!("rg --files failed: {error}")))?;
        if !output.status.success() && output.status.code() != Some(1) {
            return Err(ToolError::ExecutionFailed(format!(
                "rg --files exited with status {}: {}",
                output
                    .status
                    .code()
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let max_results = input.max_results.unwrap_or(500);
        let paths = String::from_utf8_lossy(&output.stdout)
            .lines()
            .take(max_results)
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        json_observation(
            self.name(),
            &SearchGlobOutput {
                root: root.display().to_string(),
                patterns: input.patterns,
                paths,
            },
        )
    }
}

impl Tool for SearchPathTool {
    fn name(&self) -> &str {
        "search.path"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: SearchPathInput = parse_tool_input(request.input)?;
        let patterns = normalize_patterns(input.patterns)?;
        let root =
            resolve_optional_directory(context, input.root.as_deref())?.unwrap_or_else(default_cwd);
        let entry_type = input.entry_type.unwrap_or(SearchPathEntryType::Any);
        let include_hidden = input.include_hidden.unwrap_or(false);
        let max_depth = input.max_depth.unwrap_or(8);
        let max_results = input.max_results.unwrap_or(500);
        let mut entries = Vec::new();
        collect_matching_paths(
            &root,
            &root,
            0,
            max_depth,
            include_hidden,
            entry_type,
            &patterns,
            max_results,
            &mut entries,
        )?;
        json_observation(
            self.name(),
            &SearchPathOutput {
                root: root.display().to_string(),
                patterns,
                entry_type: entry_type.as_str(),
                entries,
            },
        )
    }
}

fn normalize_patterns(patterns: Vec<String>) -> Result<Vec<String>, ToolError> {
    let normalized = patterns
        .into_iter()
        .map(|pattern| pattern.trim().to_string())
        .filter(|pattern| !pattern.is_empty())
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return Err(ToolError::ExecutionFailed(
            "patterns must contain at least one value".into(),
        ));
    }
    Ok(normalized)
}

#[allow(clippy::too_many_arguments)]
fn collect_matching_paths(
    root: &Path,
    path: &Path,
    depth: usize,
    max_depth: usize,
    include_hidden: bool,
    entry_type: SearchPathEntryType,
    patterns: &[String],
    max_results: usize,
    entries: &mut Vec<SearchPathEntry>,
) -> Result<(), ToolError> {
    if entries.len() >= max_results {
        return Ok(());
    }
    let read_dir = fs::read_dir(path)
        .map_err(|error| ToolError::ExecutionFailed(format!("path search failed: {error}")))?;
    for item in read_dir {
        if entries.len() >= max_results {
            break;
        }
        let item = item
            .map_err(|error| ToolError::ExecutionFailed(format!("path entry failed: {error}")))?;
        let name = item.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') {
            continue;
        }
        let item_path = item.path();
        let metadata = fs::symlink_metadata(&item_path).map_err(|error| {
            ToolError::ExecutionFailed(format!("path metadata failed: {error}"))
        })?;
        let is_file = metadata.is_file();
        let is_directory = metadata.is_dir();
        let is_symlink = metadata.file_type().is_symlink();
        if entry_type.matches(is_file, is_directory) {
            let relative_path = item_path
                .strip_prefix(root)
                .unwrap_or(item_path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            if patterns
                .iter()
                .any(|pattern| path_pattern_matches(pattern, &relative_path, &name))
            {
                entries.push(SearchPathEntry {
                    path: relative_path,
                    name: name.clone(),
                    is_file,
                    is_directory,
                    is_symlink,
                });
            }
        }
        if is_directory && depth < max_depth {
            collect_matching_paths(
                root,
                &item_path,
                depth + 1,
                max_depth,
                include_hidden,
                entry_type,
                patterns,
                max_results,
                entries,
            )?;
        }
    }
    Ok(())
}

fn path_pattern_matches(pattern: &str, relative_path: &str, name: &str) -> bool {
    wildcard_matches(pattern, relative_path) || wildcard_matches(pattern, name)
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let value_chars = value.chars().collect::<Vec<_>>();
    let (mut pattern_index, mut value_index) = (0usize, 0usize);
    let mut star_index = None;
    let mut star_match_index = 0usize;
    while value_index < value_chars.len() {
        if pattern_index < pattern_chars.len()
            && (pattern_chars[pattern_index] == '?'
                || pattern_chars[pattern_index] == value_chars[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
            star_index = Some(pattern_index);
            star_match_index = value_index;
            pattern_index += 1;
        } else if let Some(index) = star_index {
            pattern_index = index + 1;
            star_match_index += 1;
            value_index = star_match_index;
        } else {
            return false;
        }
    }
    while pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
        pattern_index += 1;
    }
    pattern_index == pattern_chars.len()
}

impl SearchPathEntryType {
    fn matches(self, is_file: bool, is_directory: bool) -> bool {
        match self {
            Self::Any => true,
            Self::File => is_file,
            Self::Directory => is_directory,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::File => "file",
            Self::Directory => "directory",
        }
    }
}
