use std::process::Command;

use serde::Deserialize;

use crate::domain::agent::{AgentRunContext, Tool, ToolError, ToolRequest};

use super::{
    SearchGlobOutput, SearchGrepOutput, default_cwd, json_observation, parse_tool_input,
    resolve_optional_directory,
};

pub(super) struct SearchGrepTool;
pub(super) struct SearchGlobTool;

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
        let root = resolve_optional_directory(context, input.root.as_deref())?
            .unwrap_or_else(default_cwd);
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
        command.arg("-m").arg(input.max_results.unwrap_or(200).to_string());
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
        let root = resolve_optional_directory(context, input.root.as_deref())?
            .unwrap_or_else(default_cwd);
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
