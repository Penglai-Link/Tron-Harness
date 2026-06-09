mod common;

use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tron_backend::config::AppConfig;

use common::{
    app, app_with_config, authenticated_bearer_token, authenticated_request, request_with_headers,
    response_json,
};

static DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct ProbeServer {
    base_url: String,
    _task: JoinHandle<()>,
}

async fn mcp_probe_server() -> ProbeServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("probe server should bind");
    let address = listener
        .local_addr()
        .expect("probe server should expose a local address");
    let app = Router::new()
        .route("/mcp", post(probe_mcp));
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("probe server should remain available");
    });
    ProbeServer {
        base_url: format!("http://{address}"),
        _task: task,
    }
}

async fn probe_mcp(Json(body): Json<Value>) -> Json<Value> {
    let method = body
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let id = body.get("id").cloned();

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-06-18",
                "serverInfo": {
                    "name": "tron-mcp-probe",
                    "version": "1.0.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{
                    "name": "search_docs",
                    "description": "Search the knowledge probe.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string"
                            }
                        },
                        "required": ["query"]
                    }
                }]
            }
        }),
        "tools/call" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{
                    "type": "text",
                    "text": body
                        .get("params")
                        .and_then(|params| params.get("arguments"))
                        .and_then(|arguments| arguments.get("query"))
                        .and_then(Value::as_str)
                        .map(|query| format!("probe:{query}"))
                        .unwrap_or_else(|| "probe".to_string())
                }]
            }
        }),
        "notifications/initialized" => json!({}),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
    };

    Json(response)
}

#[tokio::test]
async fn gateway_config_persists_and_transport_enablement_is_enforced() {
    let router = app();
    let transport_token = authenticated_bearer_token(&router).await;

    let get_response =
        authenticated_request(&router, Method::GET, "/api/v1/mcp/gateway", None).await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_json = response_json(get_response).await;
    assert_eq!(get_json["data"]["gatewayEnabled"], json!(false));
    assert_eq!(get_json["data"]["variablesEndpointEnabled"], json!(false));

    let disabled_transport = request_with_headers(
        &router,
        Method::POST,
        "/mcp/variables",
        Some(json!({
            "jsonrpc": "2.0",
            "id": "disabled",
            "method": "ping"
        })),
        None,
        Some(&format!("Bearer {transport_token}")),
    )
    .await;
    assert_eq!(disabled_transport.status(), StatusCode::SERVICE_UNAVAILABLE);
    let disabled_transport_json = response_json(disabled_transport).await;
    assert_eq!(disabled_transport_json["error"]["code"], json!(-32002));

    let patch_response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/mcp/gateway",
        Some(json!({
            "gatewayEnabled": true,
            "variablesEndpointEnabled": true
        })),
    )
    .await;
    assert_eq!(patch_response.status(), StatusCode::OK);
    let patch_json = response_json(patch_response).await;
    assert_eq!(patch_json["data"]["gatewayEnabled"], json!(true));
    assert_eq!(patch_json["data"]["variablesEndpointEnabled"], json!(true));

    let unauthorized_transport = request_with_headers(
        &router,
        Method::POST,
        "/mcp/variables",
        Some(json!({
            "jsonrpc": "2.0",
            "id": "unauthorized",
            "method": "ping"
        })),
        None,
        None,
    )
    .await;
    assert_eq!(unauthorized_transport.status(), StatusCode::UNAUTHORIZED);

    let enabled_transport = request_with_headers(
        &router,
        Method::POST,
        "/mcp/variables",
        Some(json!({
            "jsonrpc": "2.0",
            "id": "ping-1",
            "method": "ping"
        })),
        None,
        Some(&format!("Bearer {transport_token}")),
    )
    .await;
    assert_eq!(enabled_transport.status(), StatusCode::OK);
    let enabled_transport_json = response_json(enabled_transport).await;
    assert_eq!(enabled_transport_json["result"], json!({}));
}

#[tokio::test]
async fn platform_registry_health_results_and_delete_cascade_persist() {
    let database_url = temp_database_url("mcp-health-cascade");
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let healthy_probe = mcp_probe_server().await;

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "knowledge_tools",
            "name": "Knowledge Tools",
            "description": "Workspace knowledge MCP server",
            "url": format!("{}/mcp", healthy_probe.base_url),
            "authKind": "none",
            "headers": {}
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let healthy_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers/knowledge_tools/health",
        None,
    )
    .await;
    assert_eq!(healthy_response.status(), StatusCode::OK);
    let healthy_json = response_json(healthy_response).await;
    assert_eq!(healthy_json["data"]["lastHealthStatus"], json!("HEALTHY"));
    assert!(healthy_json["data"]["lastHealthCheckedAt"].is_string());

    let broken_create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "broken_tools",
            "name": "Broken Tools",
            "description": "Broken MCP server",
            "url": "http://127.0.0.1:9/health",
            "authKind": "none",
            "headers": {}
        })),
    )
    .await;
    assert_eq!(broken_create.status(), StatusCode::CREATED);

    let unhealthy_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers/broken_tools/health",
        None,
    )
    .await;
    assert_eq!(unhealthy_response.status(), StatusCode::BAD_GATEWAY);
    let unhealthy_json = response_json(unhealthy_response).await;
    assert_eq!(unhealthy_json["code"], json!("MCP_VERIFICATION_FAILED"));

    let broken_detail = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/mcp/servers/broken_tools",
        None,
    )
    .await;
    assert_eq!(broken_detail.status(), StatusCode::OK);
    let broken_detail_json = response_json(broken_detail).await;
    assert_eq!(
        broken_detail_json["data"]["lastHealthStatus"],
        json!("UNHEALTHY")
    );
    assert!(broken_detail_json["data"]["lastHealthMessage"].is_string());

    let workspace_disable = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/workspaces/TRON_CORE/servers/knowledge_tools/disable",
        None,
    )
    .await;
    assert_eq!(workspace_disable.status(), StatusCode::OK);
    let workspace_disable_json = response_json(workspace_disable).await;
    assert_eq!(
        workspace_disable_json["data"]["workspaceOverrideDisabled"],
        json!(true)
    );
    assert_eq!(
        workspace_disable_json["data"]["effectiveEnabled"],
        json!(false)
    );

    let delete_response = authenticated_request(
        &router,
        Method::DELETE,
        "/api/v1/mcp/servers/knowledge_tools",
        None,
    )
    .await;
    assert_eq!(delete_response.status(), StatusCode::OK);
    let delete_json = response_json(delete_response).await;
    assert_eq!(delete_json["data"]["deleted"], json!(true));

    let workspace_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/mcp/workspaces/TRON_CORE/servers",
        None,
    )
    .await;
    assert_eq!(workspace_list.status(), StatusCode::OK);
    let workspace_list_json = response_json(workspace_list).await;
    let rows = workspace_list_json["data"]
        .as_array()
        .expect("workspace MCP list should return an array");
    assert!(
        rows.iter()
            .all(|row| row["serverCode"] != json!("knowledge_tools")),
        "deleting a platform server should cascade owned workspace override rows",
    );
}

#[tokio::test]
async fn workspace_effective_servers_follow_override_and_platform_disable_rules() {
    let database_url = temp_database_url("mcp-effective-rules");
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let healthy_probe = mcp_probe_server().await;

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "memory_tools",
            "name": "Memory Tools",
            "description": "Workspace memory MCP server",
            "url": format!("{}/mcp", healthy_probe.base_url),
            "authKind": "bearer_static",
            "authPayload": {
                "tokenRef": "MEMORY_TOKEN"
            },
            "headers": {
                "X-Workspace": "TRON_CORE"
            }
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/mcp/workspaces/TRON_CORE/servers",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let initial_row = list_json["data"]
        .as_array()
        .expect("workspace list should be an array")
        .iter()
        .find(|row| row["serverCode"] == json!("memory_tools"))
        .cloned()
        .expect("created server should appear in workspace effective list");
    assert_eq!(initial_row["platformEnabled"], json!(true));
    assert_eq!(initial_row["effectiveEnabled"], json!(true));
    assert_eq!(initial_row.get("url"), None);
    assert_eq!(initial_row.get("authKind"), None);
    assert_eq!(initial_row.get("headers"), None);

    let disable_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/workspaces/TRON_CORE/servers/memory_tools/disable",
        None,
    )
    .await;
    assert_eq!(disable_response.status(), StatusCode::OK);
    let disable_json = response_json(disable_response).await;
    assert_eq!(
        disable_json["data"]["workspaceOverrideDisabled"],
        json!(true)
    );
    assert_eq!(disable_json["data"]["effectiveEnabled"], json!(false));

    let enable_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/workspaces/TRON_CORE/servers/memory_tools/enable",
        None,
    )
    .await;
    assert_eq!(enable_response.status(), StatusCode::OK);
    let enable_json = response_json(enable_response).await;
    assert_eq!(
        enable_json["data"]["workspaceOverrideDisabled"],
        json!(false)
    );
    assert_eq!(enable_json["data"]["effectiveEnabled"], json!(true));

    let platform_disable = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers/memory_tools/disable",
        None,
    )
    .await;
    assert_eq!(platform_disable.status(), StatusCode::OK);

    let platform_disabled_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/mcp/workspaces/TRON_CORE/servers",
        None,
    )
    .await;
    assert_eq!(platform_disabled_list.status(), StatusCode::OK);
    let platform_disabled_json = response_json(platform_disabled_list).await;
    let disabled_row = platform_disabled_json["data"]
        .as_array()
        .expect("workspace list should be an array")
        .iter()
        .find(|row| row["serverCode"] == json!("memory_tools"))
        .cloned()
        .expect("disabled platform server should remain visible to the workspace");
    assert_eq!(disabled_row["platformEnabled"], json!(false));
    assert_eq!(disabled_row["effectiveEnabled"], json!(false));

    let denied_enable = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/workspaces/TRON_CORE/servers/memory_tools/enable",
        None,
    )
    .await;
    assert_eq!(denied_enable.status(), StatusCode::CONFLICT);
    let denied_enable_json = response_json(denied_enable).await;
    assert_eq!(
        denied_enable_json["code"],
        json!("MCP_EFFECTIVE_ENABLE_DENIED")
    );
}

#[tokio::test]
async fn platform_server_draft_verify_reports_health_without_persisting() {
    let database_url = temp_database_url("mcp-draft-verify");
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let healthy_probe = mcp_probe_server().await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/mcp/servers/verify",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "definition": {
                "serverCode": "draft_tools",
                "name": "Draft Tools",
                "description": "Draft MCP server",
                "transportKind": "streamable_http",
            "url": format!("{}/mcp", healthy_probe.base_url),
                "authKind": "none",
                "headers": {},
                "headersVariableRefs": {},
                "command": "",
                "args": [],
                "envVariableRefs": {},
                "envPassthroughRefs": [],
                "workingDirectory": null
            }
        })),
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_json = response_json(verify_response).await;
    assert_eq!(verify_json["data"]["lastHealthStatus"], json!("HEALTHY"));
    assert!(verify_json["data"]["lastHealthCheckedAt"].is_string());

    let list_response = authenticated_request(&router, Method::GET, "/api/v1/mcp/servers", None).await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let rows = list_json["data"]
        .as_array()
        .expect("server list should be an array");
    assert!(
        rows.iter().all(|row| row["serverCode"] != json!("draft_tools")),
        "draft verify must not persist the server definition",
    );
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let process_id = std::process::id();
    let sequence = DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "tron-mcp-{test_name}-{process_id}-{nanos}-{sequence}.sqlite3"
    ))
}
