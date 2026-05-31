use std::{future::Future, path::Path, pin::Pin, process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time::timeout,
};

use crate::domain::agent::{
    AgentModelConfig, ModelRequest, ModelResponse, ModelStreamEvent, ModelStreamEventKind,
};

use super::{
    ModelClient, ModelStreamSink,
    system_cli_output_parser::{ParsedSystemAgentOutput, parse_complete_stdout, parse_stream_line},
};

const DEFAULT_SYSTEM_AGENT_TIMEOUT_MS: u64 = 180_000;
const STREAM_DELTA_CHUNK_CHARS: usize = 72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAgentCliKind {
    Codex,
    OpenCode,
}

impl SystemAgentCliKind {
    pub fn from_agent_code(agent_code: &str) -> Option<Self> {
        match agent_code.trim().to_ascii_lowercase().as_str() {
            "codex" => Some(Self::Codex),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemAgentCliModelClient {
    command_name: String,
    kind: SystemAgentCliKind,
    working_directory: Option<String>,
}

impl SystemAgentCliModelClient {
    pub fn new(
        kind: SystemAgentCliKind,
        command_name: impl Into<String>,
        working_directory: Option<String>,
    ) -> Self {
        Self {
            command_name: command_name.into(),
            kind,
            working_directory,
        }
    }

    async fn invoke(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let (mut command, timeout_ms) = self.build_command(config, request)?;
        let output = timeout(Duration::from_millis(timeout_ms), command.output())
            .await
            .map_err(|_| {
                format!(
                    "{} command timed out after {}ms",
                    system_agent_name(self.kind),
                    timeout_ms
                )
            })?
            .map_err(|error| {
                format!(
                    "{} command failed to start: {error}",
                    system_agent_name(self.kind)
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(format_command_failure(
                self.kind,
                output.status.code(),
                &stdout,
                &stderr,
            ));
        }

        let parsed = parse_complete_stdout(self.kind, &stdout);
        self.model_response_from_parsed(parsed, stdout, request)
    }

    fn build_command(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<(Command, u64), String> {
        let prompt = prompt_from_request(request);
        if prompt.is_empty() {
            return Err("prompt is required".into());
        }
        let timeout_ms = config.timeout_ms.unwrap_or(DEFAULT_SYSTEM_AGENT_TIMEOUT_MS);
        let resume_session_id = request
            .system_agent_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let mut command = Command::new(&self.command_name);
        command.env("NO_COLOR", "1");
        if let Some(working_directory) = self.working_directory.as_deref()
            && Path::new(working_directory).exists()
        {
            command.current_dir(working_directory);
        }
        match self.kind {
            SystemAgentCliKind::Codex => {
                command.arg("exec");
                if resume_session_id.is_some() {
                    command.arg("resume");
                }
                command.arg("--skip-git-repo-check");
                command.arg("--json");
                if let Some(model_name) = normalized_value(Some(config.model_name.clone())) {
                    command.arg("--model");
                    command.arg(model_name);
                }
                if let Some(working_directory) = self.working_directory.as_deref() {
                    command.arg("--cd");
                    command.arg(working_directory);
                }
                if let Some(session_id) = resume_session_id {
                    command.arg(session_id);
                }
                command.arg(prompt);
            }
            SystemAgentCliKind::OpenCode => {
                command.arg("run");
                command.arg("--format");
                command.arg("json");
                command.arg("--thinking");
                if let Some(session_id) = resume_session_id {
                    command.arg("--session");
                    command.arg(session_id);
                }
                if let Some(working_directory) = self.working_directory.as_deref() {
                    command.arg("--dir");
                    command.arg(working_directory);
                }
                if config.model_name.contains('/') {
                    command.arg("--model");
                    command.arg(config.model_name.as_str());
                }
                command.arg(prompt);
            }
        }
        Ok((command, timeout_ms))
    }

    async fn invoke_stream(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        sink: ModelStreamSink,
    ) -> Result<ModelResponse, String> {
        let (mut command, timeout_ms) = self.build_command(config, request)?;
        command.kill_on_drop(true);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            format!(
                "{} command failed to start: {error}",
                system_agent_name(self.kind)
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            format!(
                "{} command did not expose stdout stream",
                system_agent_name(self.kind)
            )
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            format!(
                "{} command did not expose stderr stream",
                system_agent_name(self.kind)
            )
        })?;
        let kind = self.kind;
        let sink_for_stdout = sink.clone();
        let stdout_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            let mut stream_output = CliStreamOutput::default();
            while let Some(line) = lines
                .next_line()
                .await
                .map_err(|error| format!("{kind:?} stdout read failed: {error}"))?
            {
                stream_output.stdout.push_str(&line);
                stream_output.stdout.push('\n');
                let events = parse_stream_line(kind, line.as_str(), &mut stream_output.parsed);
                for event in events {
                    if event.kind == ModelStreamEventKind::Text {
                        stream_output.emitted_text_delta = true;
                    }
                    sink_for_stdout(event);
                }
            }
            Ok::<CliStreamOutput, String>(stream_output)
        });
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            let mut stderr_buffer = String::new();
            while let Some(line) = lines
                .next_line()
                .await
                .map_err(|error| format!("{kind:?} stderr read failed: {error}"))?
            {
                stderr_buffer.push_str(&line);
                stderr_buffer.push('\n');
            }
            Ok::<String, String>(stderr_buffer)
        });
        let status = match timeout(Duration::from_millis(timeout_ms), child.wait()).await {
            Ok(result) => result.map_err(|error| {
                format!(
                    "{} command failed while waiting: {error}",
                    system_agent_name(self.kind)
                )
            })?,
            Err(_) => {
                let _ = child.kill().await;
                return Err(format!(
                    "{} command timed out after {}ms",
                    system_agent_name(self.kind),
                    timeout_ms
                ));
            }
        };
        let stream_output = stdout_task.await.map_err(|error| {
            format!(
                "{} stdout task join failed: {error}",
                system_agent_name(self.kind)
            )
        })??;
        let stderr = stderr_task.await.map_err(|error| {
            format!(
                "{} stderr task join failed: {error}",
                system_agent_name(self.kind)
            )
        })??;
        if !status.success() {
            return Err(format_command_failure(
                self.kind,
                status.code(),
                &stream_output.stdout,
                &stderr,
            ));
        }
        if !stream_output.emitted_text_delta
            && let Some(output) = stream_output.parsed.final_output.clone()
            && !output.is_empty()
        {
            for chunk in split_stream_chunks(output.as_str(), STREAM_DELTA_CHUNK_CHARS) {
                sink(ModelStreamEvent::text_delta(chunk));
            }
        }
        self.model_response_from_parsed(stream_output.parsed, stream_output.stdout, request)
    }

    fn model_response_from_parsed(
        &self,
        parsed: ParsedSystemAgentOutput,
        stdout: String,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let final_output = parsed
            .final_output
            .or_else(|| normalized_value(Some(stdout.clone())))
            .ok_or_else(|| {
                format!(
                    "{} command returned no assistant output",
                    system_agent_name(self.kind)
                )
            })?;

        Ok(ModelResponse::final_output_with_usage_and_session(
            final_output,
            parsed.token_usage,
            parsed
                .system_agent_session_id
                .or_else(|| request.system_agent_session_id.clone()),
        ))
    }
}

impl ModelClient for SystemAgentCliModelClient {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move { self.invoke(config, request).await })
    }

    fn generate_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move { self.invoke_stream(config, request, sink).await })
    }
}

#[derive(Debug, Default)]
struct CliStreamOutput {
    emitted_text_delta: bool,
    parsed: ParsedSystemAgentOutput,
    stdout: String,
}

fn format_command_failure(
    kind: SystemAgentCliKind,
    status_code: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> String {
    let diagnostic = normalized_value(Some(stderr.to_string()))
        .or_else(|| normalized_value(Some(stdout.to_string())))
        .unwrap_or_else(|| "no diagnostics available".into());
    let status = status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".into());
    format!(
        "{} command failed with exit code {}: {}",
        system_agent_name(kind),
        status,
        diagnostic
    )
}

fn normalized_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn split_stream_chunks(value: &str, max_chars: usize) -> Vec<String> {
    if value.is_empty() || max_chars == 0 {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;
    for character in value.chars() {
        current.push(character);
        current_chars += 1;
        if current_chars >= max_chars {
            chunks.push(current.clone());
            current.clear();
            current_chars = 0;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn prompt_from_request(request: &ModelRequest) -> String {
    request
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            (message.role == "user")
                .then(|| message.content.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| {
            request
                .messages
                .last()
                .map(|message| message.content.trim().to_string())
                .unwrap_or_default()
        })
}

fn system_agent_name(kind: SystemAgentCliKind) -> &'static str {
    match kind {
        SystemAgentCliKind::Codex => "codex",
        SystemAgentCliKind::OpenCode => "opencode",
    }
}
