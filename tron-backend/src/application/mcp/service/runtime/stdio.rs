use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

use serde_json::{Value, json};

use crate::domain::mcp::{McpError, PlatformMcpServerRecord};

use super::DiscoveredMcpTool;

#[cfg(test)]
mod stdio_tests;

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
static JSONRPC_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
const STDIO_PROTOCOL_CANDIDATES: [StdioProtocolMode; 2] = [
    StdioProtocolMode::ContentLength,
    StdioProtocolMode::JsonLine,
];

#[cfg(test)]
const STDIO_INITIALIZE_TIMEOUT: Duration = Duration::from_millis(1500);
#[cfg(not(test))]
const STDIO_INITIALIZE_TIMEOUT: Duration = Duration::from_secs(3);

#[cfg(test)]
const STDIO_TOOLS_LIST_TIMEOUT: Duration = Duration::from_millis(1500);
#[cfg(not(test))]
const STDIO_TOOLS_LIST_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
const STDIO_TOOLS_CALL_TIMEOUT: Duration = Duration::from_millis(1000);
#[cfg(not(test))]
const STDIO_TOOLS_CALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(super) struct StdioDiscovery {
    pub protocol_mode: StdioProtocolMode,
    pub tools: Vec<DiscoveredMcpTool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdioProtocolMode {
    ContentLength,
    JsonLine,
}

pub(super) async fn discover_stdio_tools(
    server: &PlatformMcpServerRecord,
    env: &BTreeMap<String, String>,
) -> Result<StdioDiscovery, McpError> {
    let server = server.clone();
    let env = env.clone();
    tokio::task::spawn_blocking(move || discover_stdio_tools_sync(&server, &env))
        .await
        .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?
}

pub(super) async fn invoke_stdio_tool_call(
    server_code: String,
    remote_tool_name: String,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    working_directory: Option<String>,
    protocol_mode: StdioProtocolMode,
    input: Value,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        invoke_stdio_tool_call_sync(
            server_code,
            remote_tool_name,
            command,
            args,
            env,
            working_directory,
            protocol_mode,
            input,
        )
    })
    .await
    .map_err(|error| error.to_string())?
}

pub(super) async fn verify_stdio_server(
    server: &PlatformMcpServerRecord,
    env: &BTreeMap<String, String>,
) -> Result<(), String> {
    let server = server.clone();
    let env = env.clone();
    tokio::task::spawn_blocking(move || verify_stdio_server_sync(&server, &env))
        .await
        .map_err(|error| error.to_string())?
}

fn discover_stdio_tools_sync(
    server: &PlatformMcpServerRecord,
    env: &BTreeMap<String, String>,
) -> Result<StdioDiscovery, McpError> {
    let mut session =
        StdioSession::spawn_initialized(server, env).map_err(McpError::RuntimeDiscoveryFailed)?;
    session
        .send_notification("notifications/initialized")
        .map_err(McpError::RuntimeDiscoveryFailed)?;
    let result = session
        .request("tools/list", json!({}), STDIO_TOOLS_LIST_TIMEOUT)
        .map_err(McpError::RuntimeDiscoveryFailed)?;
    let tools = parse_discovered_tools(&result).map_err(McpError::RuntimeDiscoveryFailed)?;
    Ok(StdioDiscovery {
        protocol_mode: session.protocol_mode(),
        tools,
    })
}

fn invoke_stdio_tool_call_sync(
    server_code: String,
    remote_tool_name: String,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    working_directory: Option<String>,
    protocol_mode: StdioProtocolMode,
    input: Value,
) -> Result<String, String> {
    let mut session = StdioSession::spawn_initialized_raw_with_mode(
        command,
        args,
        env,
        working_directory,
        protocol_mode,
    )?;
    session.send_notification("notifications/initialized")?;
    let result = session.request(
        "tools/call",
        json!({
            "name": remote_tool_name,
            "arguments": input,
        }),
        STDIO_TOOLS_CALL_TIMEOUT,
    )?;
    let _ = server_code;
    Ok(mcp_tool_result_to_output(&result))
}

fn verify_stdio_server_sync(
    server: &PlatformMcpServerRecord,
    env: &BTreeMap<String, String>,
) -> Result<(), String> {
    let _ = discover_stdio_tools_sync(server, env).map_err(mcp_error_message)?;
    Ok(())
}

struct StdioSession {
    child: Child,
    stdin: ChildStdin,
    protocol_mode: StdioProtocolMode,
    messages: Receiver<Result<Value, String>>,
}

impl StdioSession {
    fn spawn_initialized(
        server: &PlatformMcpServerRecord,
        env: &BTreeMap<String, String>,
    ) -> Result<Self, String> {
        Self::spawn_initialized_raw(
            server.command.clone(),
            server.args.clone(),
            env.clone(),
            server.working_directory.clone(),
        )
    }

    fn spawn_initialized_raw(
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        working_directory: Option<String>,
    ) -> Result<Self, String> {
        let supported = STDIO_PROTOCOL_CANDIDATES
            .iter()
            .map(|mode| mode.label())
            .collect::<Vec<_>>()
            .join(", ");
        let mut errors = Vec::new();
        for protocol_mode in STDIO_PROTOCOL_CANDIDATES {
            let mut session = Self::spawn_raw_with_mode(
                command.clone(),
                args.clone(),
                env.clone(),
                working_directory.clone(),
                protocol_mode,
            )?;
            match session.initialize() {
                Ok(()) => return Ok(session),
                Err(error) => errors.push(format!("{}: {error}", protocol_mode.label())),
            }
        }
        Err(format!(
            "stdio MCP initialize failed for supported protocols [{supported}]: {}",
            errors.join("; ")
        ))
    }

    fn spawn_initialized_raw_with_mode(
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        working_directory: Option<String>,
        protocol_mode: StdioProtocolMode,
    ) -> Result<Self, String> {
        let mut session =
            Self::spawn_raw_with_mode(command, args, env, working_directory, protocol_mode)?;
        session.initialize()?;
        Ok(session)
    }

    fn spawn_raw_with_mode(
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        working_directory: Option<String>,
        protocol_mode: StdioProtocolMode,
    ) -> Result<Self, String> {
        let mut child = Command::new(command)
            .args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(
                working_directory
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(".")),
            )
            .spawn()
            .map_err(|error| format!("failed to spawn stdio MCP server: {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "stdio MCP server did not expose stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "stdio MCP server did not expose stdout".to_string())?;
        Ok(Self {
            child,
            stdin,
            protocol_mode,
            messages: spawn_stdout_reader(stdout, protocol_mode),
        })
    }

    fn protocol_mode(&self) -> StdioProtocolMode {
        self.protocol_mode
    }

    fn initialize(&mut self) -> Result<(), String> {
        let _ = self.request("initialize", initialize_params(), STDIO_INITIALIZE_TIMEOUT)?;
        Ok(())
    }

    fn send_notification(&mut self, method: &str) -> Result<(), String> {
        self.write_json(&json!({
            "jsonrpc": "2.0",
            "method": method,
        }))
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = jsonrpc_request_id();
        self.write_json(&json!({
            "jsonrpc": "2.0",
            "id": id.clone(),
            "method": method,
            "params": params,
        }))?;

        loop {
            let response = self.read_json(method, timeout)?;
            if response.get("id") != Some(&id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                let code = error.get("code").and_then(Value::as_i64).unwrap_or(-32603);
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("stdio MCP request failed");
                let detail = error
                    .get("data")
                    .filter(|value| !value.is_null())
                    .map(|value| format!(": {value}"))
                    .unwrap_or_default();
                return Err(format!(
                    "stdio MCP `{method}` failed ({code}): {message}{detail}"
                ));
            }
            return response
                .get("result")
                .cloned()
                .ok_or_else(|| format!("stdio MCP `{method}` response is missing result"));
        }
    }

    fn read_json(&mut self, method: &str, timeout: Duration) -> Result<Value, String> {
        match self.messages.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "stdio MCP `{method}` timed out after {} ms",
                timeout.as_millis()
            )),
            Err(RecvTimeoutError::Disconnected) => Err(format!(
                "stdio MCP reader disconnected before `{method}` completed"
            )),
        }
    }

    fn write_json(&mut self, value: &Value) -> Result<(), String> {
        if self.protocol_mode == StdioProtocolMode::JsonLine {
            return self.write_json_line(value);
        }

        let body = serde_json::to_vec(value)
            .map_err(|error| format!("failed to serialize stdio MCP request: {error}"))?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|error| format!("failed to write stdio MCP header: {error}"))?;
        self.stdin
            .write_all(&body)
            .map_err(|error| format!("failed to write stdio MCP body: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("failed to flush stdio MCP request: {error}"))
    }

    fn write_json_line(&mut self, value: &Value) -> Result<(), String> {
        let body = serde_json::to_vec(value)
            .map_err(|error| format!("failed to serialize stdio MCP request: {error}"))?;
        self.stdin
            .write_all(&body)
            .map_err(|error| format!("failed to write stdio MCP json line: {error}"))?;
        self.stdin
            .write_all(b"\n")
            .map_err(|error| format!("failed to terminate stdio MCP json line: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("failed to flush stdio MCP request: {error}"))
    }
}

impl Drop for StdioSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl StdioProtocolMode {
    fn label(self) -> &'static str {
        match self {
            Self::ContentLength => "content-length",
            Self::JsonLine => "json-line",
        }
    }
}

fn spawn_stdout_reader(
    stdout: ChildStdout,
    protocol_mode: StdioProtocolMode,
) -> Receiver<Result<Value, String>> {
    let (sender, receiver) = mpsc::sync_channel(32);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let message = match protocol_mode {
                StdioProtocolMode::ContentLength => read_content_length_json(&mut reader),
                StdioProtocolMode::JsonLine => read_json_line(&mut reader),
            };
            let terminal = message.is_err();
            if sender.send(message).is_err() {
                break;
            }
            if terminal {
                break;
            }
        }
    });
    receiver
}

fn read_content_length_json<R: BufRead + Read>(reader: &mut R) -> Result<Value, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read stdio MCP header: {error}"))?;
        if bytes == 0 {
            return Err("stdio MCP server closed stdout unexpectedly".to_string());
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| format!("invalid stdio MCP content length: {error}"))?,
            );
        }
    }

    let content_length =
        content_length.ok_or_else(|| "stdio MCP response is missing Content-Length".to_string())?;
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("failed to read stdio MCP body: {error}"))?;
    serde_json::from_slice(&body)
        .map_err(|error| format!("invalid stdio MCP JSON response: {error}"))
}

fn read_json_line<R: BufRead>(reader: &mut R) -> Result<Value, String> {
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read stdio MCP json line: {error}"))?;
        if bytes == 0 {
            return Err("stdio MCP server closed stdout unexpectedly".to_string());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return serde_json::from_str(trimmed).map_err(|error| {
            format!(
                "invalid stdio MCP json line: {} ({error})",
                compact_error_output(trimmed)
            )
        });
    }
}

fn parse_discovered_tools(result: &Value) -> Result<Vec<DiscoveredMcpTool>, String> {
    let Some(tools) = result.get("tools").and_then(Value::as_array) else {
        return Err("MCP tools/list response is missing tools array".to_string());
    };

    let mut discovered = Vec::with_capacity(tools.len());
    for tool in tools {
        let Some(remote_tool_name) = tool
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err("MCP tool definition is missing name".to_string());
        };
        let description = tool
            .get("description")
            .or_else(|| tool.get("title"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let input_schema = tool
            .get("inputSchema")
            .or_else(|| tool.get("input_schema"))
            .cloned()
            .unwrap_or_else(|| json!({ "type": "object" }));

        discovered.push(DiscoveredMcpTool {
            remote_tool_name: remote_tool_name.to_string(),
            description,
            input_schema,
        });
    }

    Ok(discovered)
}

fn mcp_tool_result_to_output(result: &Value) -> String {
    if let Some(contents) = result.get("content").and_then(Value::as_array) {
        let mut text_parts = Vec::new();
        for content in contents {
            if let Some(text) = content.get("text").and_then(Value::as_str) {
                text_parts.push(text.to_string());
                continue;
            }
            if let Some(text) = content.as_str() {
                text_parts.push(text.to_string());
            }
        }
        if !text_parts.is_empty() {
            return text_parts.join("\n");
        }
    }

    if let Some(structured) = result.get("structuredContent") {
        return serde_json::to_string_pretty(structured).unwrap_or_else(|_| structured.to_string());
    }

    serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string())
}

fn initialize_params() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": {
            "name": "tron-backend",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn jsonrpc_request_id() -> Value {
    Value::from(JSONRPC_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
}

fn compact_error_output(value: &str) -> String {
    const LIMIT: usize = 160;
    if value.len() <= LIMIT {
        return value.to_string();
    }
    let mut end = LIMIT.saturating_sub(3);
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}...", &value[..end])
}

fn mcp_error_message(error: McpError) -> String {
    match error {
        McpError::RuntimeDiscoveryFailed(message) | McpError::VerificationFailed(message) => {
            message
        }
        other => other.to_string(),
    }
}
