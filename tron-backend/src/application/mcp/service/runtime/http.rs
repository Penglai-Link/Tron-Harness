use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::{
    Client, StatusCode,
    header::{ACCEPT, CONTENT_TYPE},
};
use serde_json::{Value, json};

use crate::domain::mcp::{McpError, PlatformMcpServerRecord};

use super::DiscoveredMcpTool;

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
static JSONRPC_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
const HTTP_INITIALIZE_TIMEOUT: Duration = Duration::from_millis(150);
#[cfg(not(test))]
const HTTP_INITIALIZE_TIMEOUT: Duration = Duration::from_secs(3);

#[cfg(test)]
const HTTP_NOTIFICATION_TIMEOUT: Duration = Duration::from_millis(150);
#[cfg(not(test))]
const HTTP_NOTIFICATION_TIMEOUT: Duration = Duration::from_secs(3);

#[cfg(test)]
const HTTP_TOOLS_LIST_TIMEOUT: Duration = Duration::from_millis(150);
#[cfg(not(test))]
const HTTP_TOOLS_LIST_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
const HTTP_TOOLS_CALL_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(not(test))]
const HTTP_TOOLS_CALL_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn normalize_runtime_input_schema(schema: Value) -> Value {
    match schema {
        Value::Object(mut object) => {
            object
                .entry("type".to_string())
                .or_insert_with(|| Value::String("object".to_string()));
            Value::Object(object)
        }
        _ => json!({ "type": "object" }),
    }
}

pub(super) async fn discover_streamable_http_tools(
    http_client: &Client,
    server: &PlatformMcpServerRecord,
    headers: &BTreeMap<String, String>,
) -> Result<Vec<DiscoveredMcpTool>, McpError> {
    invoke_mcp_jsonrpc(
        http_client,
        &server.url,
        headers,
        "initialize",
        initialize_params(),
        HTTP_INITIALIZE_TIMEOUT,
    )
    .await?;

    send_mcp_notification(
        http_client,
        &server.url,
        headers,
        "notifications/initialized",
        HTTP_NOTIFICATION_TIMEOUT,
    )
    .await?;

    let result = invoke_mcp_jsonrpc(
        http_client,
        &server.url,
        headers,
        "tools/list",
        json!({}),
        HTTP_TOOLS_LIST_TIMEOUT,
    )
    .await?;
    parse_discovered_tools(&result)
}

pub(super) async fn verify_streamable_http_server(
    http_client: &Client,
    server: &PlatformMcpServerRecord,
    headers: &BTreeMap<String, String>,
) -> Result<(), String> {
    discover_streamable_http_tools(http_client, server, headers)
        .await
        .map(|_| ())
        .map_err(mcp_error_message)
}

pub(super) async fn call_streamable_http_tool(
    http_client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    remote_tool_name: &str,
    input: Value,
) -> Result<String, McpError> {
    let result = invoke_mcp_jsonrpc(
        http_client,
        url,
        headers,
        "tools/call",
        json!({
            "name": remote_tool_name,
            "arguments": input,
        }),
        HTTP_TOOLS_CALL_TIMEOUT,
    )
    .await?;
    Ok(mcp_tool_result_to_output(&result))
}

pub(super) async fn invoke_mcp_jsonrpc(
    http_client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, McpError> {
    let id = JSONRPC_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let headers = build_headers(headers)?;
    let (status, body) = tokio::time::timeout(timeout, async {
        let response = http_client
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .headers(headers)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }))
            .send()
            .await
            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
        Ok::<(StatusCode, String), McpError>((status, body))
    })
    .await
    .map_err(|_| mcp_timeout_error(method, timeout))??;
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(McpError::TransportUnauthorized);
    }
    if !status.is_success() {
        return Err(McpError::RuntimeDiscoveryFailed(format!(
            "MCP HTTP {status} returned {}",
            body.trim()
        )));
    }

    let response_value: Value = serde_json::from_str(&body)
        .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
    if let Some(error) = response_value.get("error") {
        let code = error.get("code").and_then(Value::as_i64).unwrap_or(-32603);
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("MCP JSON-RPC request failed");
        let detail = error
            .get("data")
            .filter(|value| !value.is_null())
            .map(|value| format!(": {value}"))
            .unwrap_or_default();
        return Err(McpError::RuntimeDiscoveryFailed(format!(
            "MCP JSON-RPC `{method}` failed ({code}): {message}{detail}"
        )));
    }

    response_value.get("result").cloned().ok_or_else(|| {
        McpError::RuntimeDiscoveryFailed(format!(
            "MCP JSON-RPC `{method}` response is missing result"
        ))
    })
}

pub(super) fn parse_discovered_tools(result: &Value) -> Result<Vec<DiscoveredMcpTool>, McpError> {
    let Some(tools) = result.get("tools").and_then(Value::as_array) else {
        return Err(McpError::RuntimeDiscoveryFailed(
            "MCP tools/list response is missing tools array".to_string(),
        ));
    };

    let mut discovered = Vec::with_capacity(tools.len());
    for tool in tools {
        let Some(remote_tool_name) = tool
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(McpError::RuntimeDiscoveryFailed(
                "MCP tool definition is missing name".to_string(),
            ));
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

pub(super) fn mcp_tool_result_to_output(result: &Value) -> String {
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

async fn send_mcp_notification(
    http_client: &Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    method: &str,
    timeout: Duration,
) -> Result<(), McpError> {
    let headers = build_headers(headers)?;
    let (status, body) = tokio::time::timeout(timeout, async {
        let response = http_client
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .headers(headers)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": method,
            }))
            .send()
            .await
            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
        Ok::<(StatusCode, String), McpError>((status, body))
    })
    .await
    .map_err(|_| mcp_timeout_error(method, timeout))??;

    if status.is_success() || status == StatusCode::ACCEPTED {
        return Ok(());
    }

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(McpError::TransportUnauthorized);
    }
    Err(McpError::RuntimeDiscoveryFailed(format!(
        "MCP notification `{method}` returned {status}: {}",
        body.trim()
    )))
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

fn build_headers(
    headers: &BTreeMap<String, String>,
) -> Result<reqwest::header::HeaderMap, McpError> {
    let mut header_map = reqwest::header::HeaderMap::new();
    for (name, value) in headers {
        let header_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
        let header_value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
        header_map.insert(header_name, header_value);
    }
    Ok(header_map)
}

fn mcp_timeout_error(method: &str, timeout: Duration) -> McpError {
    McpError::RuntimeDiscoveryFailed(format!(
        "MCP JSON-RPC `{method}` timed out after {} ms",
        timeout.as_millis()
    ))
}

fn mcp_error_message(error: McpError) -> String {
    match error {
        McpError::RuntimeDiscoveryFailed(message) | McpError::VerificationFailed(message) => {
            message
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, time::Duration};

    use axum::{Json, Router, routing::post};
    use reqwest::Client;
    use serde_json::{Value, json};
    use tokio::{net::TcpListener, task::JoinHandle, time::sleep};

    use crate::domain::mcp::PlatformMcpServerRecord;

    use super::{call_streamable_http_tool, discover_streamable_http_tools};

    struct TestHttpMcpServer {
        url: String,
        _task: JoinHandle<()>,
    }

    #[tokio::test]
    async fn streamable_http_discovery_times_out_when_initialize_stalls() {
        let server = test_http_mcp_server(http_stalled_initialize_handler).await;
        let client = Client::new();
        let error = discover_streamable_http_tools(
            &client,
            &PlatformMcpServerRecord {
                server_code: "stalled-http".into(),
                name: "stalled-http".into(),
                description: None,
                transport_kind: crate::domain::mcp::McpServerTransportKind::StreamableHttp,
                url: server.url.clone(),
                auth_kind: crate::domain::mcp::McpServerAuthKind::None,
                auth_payload: None,
                headers: BTreeMap::new(),
                headers_variable_refs: BTreeMap::new(),
                command: String::new(),
                args: Vec::new(),
                env_variable_refs: BTreeMap::new(),
                env_passthrough_refs: Vec::new(),
                working_directory: None,
                enabled: true,
                last_health_status: crate::domain::mcp::McpHealthStatus::NeverChecked,
                last_health_message: None,
                last_health_checked_at: None,
                created_at: "2026-06-09T00:00:00Z".into(),
                updated_at: "2026-06-09T00:00:00Z".into(),
            },
            &BTreeMap::new(),
        )
        .await
        .expect_err("stalled initialize should time out");

        let message = error.to_string();
        assert!(message.contains("initialize"));
        assert!(message.contains("timed out"));
    }

    #[tokio::test]
    async fn streamable_http_tool_call_timeout_is_reported() {
        let server = test_http_mcp_server(http_blocking_tool_call_handler).await;
        let client = Client::new();
        let error = call_streamable_http_tool(
            &client,
            &server.url,
            &BTreeMap::new(),
            "search_docs",
            json!({
                "query": "timeout"
            }),
        )
        .await
        .expect_err("blocking tools/call should time out");

        let message = error.to_string();
        assert!(message.contains("tools/call"));
        assert!(message.contains("timed out"));
    }

    async fn test_http_mcp_server(
        handler: fn(
            Json<Value>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Json<Value>> + Send>>,
    ) -> TestHttpMcpServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test HTTP MCP server should bind");
        let address = listener
            .local_addr()
            .expect("test HTTP MCP server should expose address");
        let app = Router::new().route("/mcp", post(handler));
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test HTTP MCP server should stay available");
        });
        TestHttpMcpServer {
            url: format!("http://{address}/mcp"),
            _task: task,
        }
    }

    fn http_stalled_initialize_handler(
        Json(payload): Json<Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Json<Value>> + Send>> {
        Box::pin(async move {
            if payload["method"] == json!("initialize") {
                sleep(Duration::from_secs(2)).await;
            }
            Json(json!({
                "jsonrpc": "2.0",
                "id": payload.get("id").cloned().unwrap_or(Value::Null),
                "result": {
                    "tools": []
                }
            }))
        })
    }

    fn http_blocking_tool_call_handler(
        Json(payload): Json<Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Json<Value>> + Send>> {
        Box::pin(async move {
            let method = payload
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match method {
                "initialize" => Json(json!({
                    "jsonrpc": "2.0",
                    "id": payload.get("id").cloned().unwrap_or(Value::Null),
                    "result": {
                        "serverInfo": {
                            "name": "blocking-tool-call",
                            "version": "1.0.0"
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                })),
                "tools/call" => {
                    sleep(Duration::from_secs(2)).await;
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": payload.get("id").cloned().unwrap_or(Value::Null),
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": "too late"
                            }]
                        }
                    }))
                }
                _ => Json(json!({
                    "jsonrpc": "2.0",
                    "id": payload.get("id").cloned().unwrap_or(Value::Null),
                    "result": {
                        "tools": []
                    }
                })),
            }
        })
    }
}
