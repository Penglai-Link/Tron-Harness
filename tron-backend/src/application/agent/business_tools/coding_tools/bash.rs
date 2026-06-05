use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use serde::Deserialize;

use crate::domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest};

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

struct StreamLine {
    line: String,
    stream: &'static str,
}

impl Tool for BashExecTool {
    fn name(&self) -> &str {
        "bash.exec"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let mut progress = |_line: String| {};
        self.exec_with_progress(request, context, &mut progress)
    }

    fn exec_with_progress(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
        progress: &mut dyn FnMut(String),
    ) -> Result<ToolExecutionResult, ToolError> {
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
        let mut child = child.spawn().map_err(|error| {
            ToolError::ExecutionFailed(format!("command failed to start: {error}"))
        })?;
        if let Some(stdin) = input.stdin {
            let mut writer = child.stdin.take().ok_or_else(|| {
                ToolError::ExecutionFailed("command stdin is not available".into())
            })?;
            writer.write_all(stdin.as_bytes()).map_err(|error| {
                ToolError::ExecutionFailed(format!("command stdin write failed: {error}"))
            })?;
        }
        let stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::ExecutionFailed("command stdout is not available".into()))?;
        let stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| ToolError::ExecutionFailed("command stderr is not available".into()))?;
        let (sender, receiver) = mpsc::channel();
        let stdout_handle = spawn_stream_reader("stdout", stdout_pipe, sender.clone());
        let stderr_handle = spawn_stream_reader("stderr", stderr_pipe, sender);

        let status = wait_for_child_with_progress(&mut child, &receiver, progress)?;
        let stdout = join_stream_reader(stdout_handle, "stdout")?;
        let stderr = join_stream_reader(stderr_handle, "stderr")?;
        drain_progress(&receiver, progress);

        if !status.success() {
            return Err(ToolError::ExecutionFailed(format!(
                "command exited with status {}.\nstdout:\n{}\nstderr:\n{}",
                status
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
                cwd: cwd.unwrap_or_else(default_cwd).display().to_string(),
                success: true,
                exit_code: status.code(),
                stdout,
                stderr,
            },
        )
    }
}

fn spawn_stream_reader<R>(
    stream: &'static str,
    reader: R,
    sender: Sender<StreamLine>,
) -> JoinHandle<Result<String, ToolError>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut collected = String::new();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).map_err(|error| {
                ToolError::ExecutionFailed(format!("command {stream} read failed: {error}"))
            })?;
            if bytes == 0 {
                break;
            }
            collected.push_str(&line);
            let display_line = line.trim_end_matches(['\r', '\n']).to_string();
            let _ = sender.send(StreamLine {
                line: display_line,
                stream,
            });
        }
        Ok(collected)
    })
}

fn wait_for_child_with_progress(
    child: &mut std::process::Child,
    receiver: &Receiver<StreamLine>,
    progress: &mut dyn FnMut(String),
) -> Result<std::process::ExitStatus, ToolError> {
    loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            ToolError::ExecutionFailed(format!("command execution failed: {error}"))
        })? {
            return Ok(status);
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => publish_stream_line(line, progress),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => thread::sleep(Duration::from_millis(20)),
        }
    }
}

fn drain_progress(receiver: &Receiver<StreamLine>, progress: &mut dyn FnMut(String)) {
    while let Ok(line) = receiver.try_recv() {
        publish_stream_line(line, progress);
    }
}

fn publish_stream_line(line: StreamLine, progress: &mut dyn FnMut(String)) {
    if line.line.trim().is_empty() {
        return;
    }
    if line.stream == "stderr" {
        progress(format!("[stderr] {}", line.line));
    } else {
        progress(line.line);
    }
}

fn join_stream_reader(
    handle: JoinHandle<Result<String, ToolError>>,
    stream: &str,
) -> Result<String, ToolError> {
    handle
        .join()
        .map_err(|_| ToolError::ExecutionFailed(format!("command {stream} reader panicked")))?
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, ToolRequest,
    };

    use super::*;

    fn test_context() -> AgentRunContext {
        AgentRunContext {
            conversation_code: None,
            run_code: "RUN-BASH-TEST".into(),
            workspace_code: "TRON_CORE".into(),
            user_code: None,
            locale: "en-US".into(),
            request_id: None,
            stream_key: None,
            agent_kind: AgentKind::React,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "MODEL-1".into(),
                protocol: AgentModelProtocol::ChatCompletions,
                base_url: "http://127.0.0.1:1/v1".into(),
                api_path: None,
                api_key: "sk-test".into(),
                model_name: "gpt-test".into(),
                thinking_enabled: false,
                responses_stateful_enabled: false,
                temperature: None,
                max_tokens: None,
                context_window_tokens: None,
                top_p: None,
                max_retries: None,
            },
            variables: Vec::new(),
            skills: Vec::new(),
            tools: Vec::new(),
            tool_exposure: Default::default(),
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![AgentMessage::user("run shell")],
            observations: Vec::new(),
            started_at: "2026-06-05T00:00:00Z".into(),
        }
    }

    #[test]
    fn bash_exec_reports_stdout_and_stderr_progress_lines() {
        let tool = BashExecTool;
        let mut progress_lines = Vec::<String>::new();
        let result = tool
            .exec_with_progress(
                ToolRequest {
                    tool_name: "bash.exec".into(),
                    input: json!({
                        "command": "sh",
                        "args": ["-c", "printf 'one\\n'; printf 'two\\n' >&2"]
                    }),
                },
                &test_context(),
                &mut |line| progress_lines.push(line),
            )
            .expect("bash command should complete");
        let output: serde_json::Value =
            serde_json::from_str(&result.observation.output).expect("output should be JSON");

        assert!(progress_lines.iter().any(|line| line == "one"));
        assert!(progress_lines.iter().any(|line| line == "[stderr] two"));
        assert_eq!(output["stdout"], json!("one\n"));
        assert_eq!(output["stderr"], json!("two\n"));
    }
}
