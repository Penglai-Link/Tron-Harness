use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;

use crate::domain::mcp::{
    McpHealthStatus, McpServerAuthKind, McpServerTransportKind, PlatformMcpServerRecord,
};

use super::{StdioProtocolMode, discover_stdio_tools, invoke_stdio_tool_call};

#[tokio::test]
async fn json_line_stdio_tools_are_discovered_without_command_name_hint() {
    let temp_dir = temp_test_dir("custom-json-line");
    let command_path = temp_dir.join("custom-mcp-server");
    fs::write(&command_path, json_line_mcp_server_script())
        .expect("json line MCP server script should be written");
    let mut permissions = fs::metadata(&command_path)
        .expect("script metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&command_path, permissions).expect("script should be executable");

    let discovery = discover_stdio_tools(
        &stdio_server_record(
            &command_path,
            &temp_dir,
            "custom-json-line",
            vec!["serve".into()],
        ),
        &BTreeMap::new(),
    )
    .await
    .expect("json line stdio tools should be discovered");

    assert_eq!(discovery.protocol_mode, StdioProtocolMode::JsonLine);
    assert_eq!(discovery.tools.len(), 1);
    assert_eq!(discovery.tools[0].remote_tool_name, "list_apps");
}

#[tokio::test]
async fn content_length_stdio_tools_are_discovered_without_command_name_hint() {
    let temp_dir = temp_test_dir("custom-content-length");
    let command_path = temp_dir.join("custom-framed-server");
    fs::write(&command_path, content_length_mcp_server_script())
        .expect("framed MCP server script should be written");
    let mut permissions = fs::metadata(&command_path)
        .expect("script metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&command_path, permissions).expect("script should be executable");

    let discovery = discover_stdio_tools(
        &stdio_server_record(
            &command_path,
            &temp_dir,
            "custom-content-length",
            Vec::new(),
        ),
        &BTreeMap::new(),
    )
    .await
    .expect("content-length stdio tools should be discovered");

    assert_eq!(discovery.protocol_mode, StdioProtocolMode::ContentLength);
    assert_eq!(discovery.tools.len(), 1);
    assert_eq!(discovery.tools[0].remote_tool_name, "read_file");
}

#[tokio::test]
async fn stdio_initialize_timeout_is_reported_when_server_never_responds() {
    let temp_dir = temp_test_dir("stdio-init-timeout");
    let command_path = temp_dir.join("hanging-server");
    fs::write(&command_path, "#!/bin/sh\nsleep 2\n")
        .expect("hanging MCP server script should be written");
    let mut permissions = fs::metadata(&command_path)
        .expect("script metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&command_path, permissions).expect("script should be executable");

    let error = discover_stdio_tools(
        &stdio_server_record(&command_path, &temp_dir, "hanging-server", Vec::new()),
        &BTreeMap::new(),
    )
    .await
    .expect_err("hanging stdio server should time out");

    let message = error.to_string();
    assert!(message.contains("initialize"));
    assert!(message.contains("timed out"));
}

#[tokio::test]
async fn stdio_tool_call_timeout_is_reported() {
    let temp_dir = temp_test_dir("stdio-tool-call-timeout");
    let command_path = temp_dir.join("blocking-tool-call");
    fs::write(
        &command_path,
        blocking_tool_call_json_line_mcp_server_script(),
    )
    .expect("blocking MCP server script should be written");
    let mut permissions = fs::metadata(&command_path)
        .expect("script metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&command_path, permissions).expect("script should be executable");

    let error = invoke_stdio_tool_call(
        "blocking-tool-call".into(),
        "list_apps".into(),
        command_path.to_string_lossy().into_owned(),
        vec!["serve".into()],
        BTreeMap::new(),
        Some(temp_dir.to_string_lossy().into_owned()),
        StdioProtocolMode::JsonLine,
        json!({
            "query": "apps"
        }),
    )
    .await
    .expect_err("blocking stdio tool call should time out");

    assert!(error.contains("tools/call"));
    assert!(error.contains("timed out"));
}

fn stdio_server_record(
    command_path: &PathBuf,
    working_directory: &PathBuf,
    server_code: &str,
    args: Vec<String>,
) -> PlatformMcpServerRecord {
    PlatformMcpServerRecord {
        server_code: server_code.into(),
        name: server_code.into(),
        description: None,
        transport_kind: McpServerTransportKind::Stdio,
        url: String::new(),
        auth_kind: McpServerAuthKind::None,
        auth_payload: None,
        headers: BTreeMap::new(),
        headers_variable_refs: BTreeMap::new(),
        command: command_path.to_string_lossy().into_owned(),
        args,
        env_variable_refs: BTreeMap::new(),
        env_passthrough_refs: Vec::new(),
        working_directory: Some(working_directory.to_string_lossy().into_owned()),
        enabled: true,
        last_health_status: McpHealthStatus::NeverChecked,
        last_health_message: None,
        last_health_checked_at: None,
        created_at: "2026-06-09T00:00:00Z".into(),
        updated_at: "2026-06-09T00:00:00Z".into(),
    }
}

fn temp_test_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    let process_id = std::process::id();
    let dir = std::env::temp_dir().join(format!("tron-{prefix}-{process_id}-{nanos}"));
    fs::create_dir_all(&dir).expect("temp test dir should be created");
    dir
}

fn json_line_mcp_server_script() -> &'static str {
    r#"#!/usr/bin/env node
const readline = require('node:readline');

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

function reply(message) {
  process.stdout.write(JSON.stringify(message) + '\n');
}

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  if (!trimmed.startsWith('{')) process.exit(1);
  const message = JSON.parse(trimmed);
  if (message.method === 'initialize') {
    reply({
      jsonrpc: '2.0',
      id: message.id,
      result: {
        protocolVersion: '2025-03-26',
        serverInfo: { name: 'open-computer-use', version: 'test' },
        capabilities: { tools: { listChanged: false } },
      },
    });
    return;
  }
  if (message.method === 'notifications/initialized') {
    return;
  }
  if (message.method === 'tools/list') {
    reply({
      jsonrpc: '2.0',
      id: message.id,
      result: {
        tools: [
          {
            name: 'list_apps',
            description: 'List apps',
            inputSchema: { type: 'object' },
          },
        ],
      },
    });
    return;
  }
  if (message.method === 'tools/call') {
    reply({
      jsonrpc: '2.0',
      id: message.id,
      result: {
        content: [
          {
            type: 'text',
            text: 'ok',
          },
        ],
      },
    });
  }
});
"#
}

fn blocking_tool_call_json_line_mcp_server_script() -> &'static str {
    r#"#!/usr/bin/env node
const readline = require('node:readline');

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

function reply(message) {
  process.stdout.write(JSON.stringify(message) + '\n');
}

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  if (!trimmed.startsWith('{')) process.exit(1);
  const message = JSON.parse(trimmed);
  if (message.method === 'initialize') {
    reply({
      jsonrpc: '2.0',
      id: message.id,
      result: {
        protocolVersion: '2025-03-26',
        serverInfo: { name: 'blocking-tool-call', version: 'test' },
        capabilities: { tools: { listChanged: false } },
      },
    });
    return;
  }
  if (message.method === 'notifications/initialized') {
    return;
  }
  if (message.method === 'tools/call') {
    setTimeout(() => {
      reply({
        jsonrpc: '2.0',
        id: message.id,
        result: {
          content: [
            {
              type: 'text',
              text: 'too late',
            },
          ],
        },
      });
    }, 2000);
  }
});
"#
}

fn content_length_mcp_server_script() -> &'static str {
    r#"#!/usr/bin/env node
let buffer = Buffer.alloc(0);

function reply(message) {
  const body = Buffer.from(JSON.stringify(message));
  process.stdout.write(`Content-Length: ${body.length}\r\n\r\n`);
  process.stdout.write(body);
}

function handle(message) {
  if (message.method === 'initialize') {
    reply({
      jsonrpc: '2.0',
      id: message.id,
      result: {
        protocolVersion: '2025-03-26',
        serverInfo: { name: 'custom-framed-server', version: 'test' },
        capabilities: { tools: { listChanged: false } },
      },
    });
    return;
  }
  if (message.method === 'notifications/initialized') {
    return;
  }
  if (message.method === 'tools/list') {
    reply({
      jsonrpc: '2.0',
      id: message.id,
      result: {
        tools: [
          {
            name: 'read_file',
            description: 'Read a file',
            inputSchema: { type: 'object' },
          },
        ],
      },
    });
  }
}

process.stdin.on('data', (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf('\r\n\r\n');
    if (headerEnd === -1) return;
    const header = buffer.slice(0, headerEnd).toString();
    const match = header.match(/content-length:\s*(\d+)/i);
    if (!match) {
      buffer = buffer.slice(headerEnd + 4);
      continue;
    }
    const length = Number(match[1]);
    const bodyStart = headerEnd + 4;
    if (buffer.length < bodyStart + length) return;
    const body = buffer.slice(bodyStart, bodyStart + length).toString();
    buffer = buffer.slice(bodyStart + length);
    handle(JSON.parse(body));
  }
});
"#
}
