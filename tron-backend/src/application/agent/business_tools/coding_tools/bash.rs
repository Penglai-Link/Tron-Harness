use std::{
    io::Write,
    process::{Command, Stdio},
};

use serde::Deserialize;

use crate::domain::agent::{AgentRunContext, Tool, ToolError, ToolRequest};

use super::{
    BashExecOutput, EnvVars, default_cwd, json_observation, parse_tool_input,
    resolve_optional_directory,
};

pub(super) struct BashExecTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BashExecInput {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    stdin: Option<String>,
    env: Option<EnvVars>,
}

impl Tool for BashExecTool {
    fn name(&self) -> &str {
        "bash.exec"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<crate::domain::agent::ToolExecutionResult, ToolError> {
        let input: BashExecInput = parse_tool_input(request.input)?;
        let command = input.command.trim();
        if command.is_empty() {
            return Err(ToolError::ExecutionFailed("command is required".into()));
        }
        let cwd = resolve_optional_directory(context, input.cwd.as_deref())?;
        let mut child = Command::new(command);
        child.args(input.args.iter());
        if let Some(directory) = cwd.as_deref() {
            child.current_dir(directory);
        }
        if let Some(env) = &input.env {
            child.envs(env.iter());
        }
        if input.stdin.is_some() {
            child.stdin(Stdio::piped());
        }
        child.stdout(Stdio::piped());
        child.stderr(Stdio::piped());
        let mut child = child
            .spawn()
            .map_err(|error| ToolError::ExecutionFailed(format!("command failed to start: {error}")))?;
        if let Some(stdin) = input.stdin {
            let mut writer = child.stdin.take().ok_or_else(|| {
                ToolError::ExecutionFailed("command stdin is not available".into())
            })?;
            writer
                .write_all(stdin.as_bytes())
                .map_err(|error| ToolError::ExecutionFailed(format!("command stdin write failed: {error}")))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| ToolError::ExecutionFailed(format!("command execution failed: {error}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(ToolError::ExecutionFailed(format!(
                "command exited with status {}.\nstdout:\n{}\nstderr:\n{}",
                output
                    .status
                    .code()
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                stdout.trim(),
                stderr.trim()
            )));
        }
        json_observation(
            self.name(),
            &BashExecOutput {
                command: command.to_string(),
                args: input.args,
                cwd: cwd
                    .unwrap_or_else(default_cwd)
                    .display()
                    .to_string(),
                success: true,
                exit_code: output.status.code(),
                stdout,
                stderr,
            },
        )
    }
}
