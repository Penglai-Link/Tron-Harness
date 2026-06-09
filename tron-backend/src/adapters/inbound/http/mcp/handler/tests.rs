use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::{app::build_router, config::AppConfig};

use super::transport::MCP_PROTOCOL_VERSION;

#[tokio::test]
async fn invalid_bearer_token_is_rejected_for_variables_transport() {
    let (config, database_path) = test_config("mcp-variables-auth");
    let router = build_router(config);

    let response = issue_json_request(
        router,
        Method::POST,
        "/mcp/variables",
        Some("invalid-token"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-test-client",
                    "version": "1.0.0"
                }
            }
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["code"], -32001);

    let _ = fs::remove_file(database_path);
}

#[tokio::test]
async fn variables_transport_reads_and_writes_effective_workspace_variables() {
    let (config, database_path) = test_config("mcp-variables-runtime");
    let router = build_router(config);

    let access_token = login(&router).await;
    let workspace_code = create_workspace(&router, &access_token).await;
    enable_gateway(&router, &access_token).await;
    create_platform_variable(&router, &access_token).await;

    let initialize = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-test-client",
                    "version": "1.0.0"
                }
            }
        }),
    )
    .await;
    assert_eq!(initialize.status(), StatusCode::OK);
    let initialize_payload = response_json(initialize).await;
    assert_eq!(
        initialize_payload["result"]["protocolVersion"],
        MCP_PROTOCOL_VERSION
    );
    assert!(initialize_payload["result"]["capabilities"]["resources"].is_object());
    assert!(initialize_payload["result"]["capabilities"]["tools"].is_object());

    let initialized = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    )
    .await;
    assert_eq!(initialized.status(), StatusCode::ACCEPTED);

    let templates = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/templates/list"
        }),
    )
    .await;
    let templates_payload = response_json(templates).await;
    assert_eq!(
        templates_payload["result"]["resourceTemplates"]
            .as_array()
            .expect("resource template list should be an array")
            .len(),
        2
    );

    let catalog_read = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/read",
            "params": {
                "uri": format!("tron://variables/workspaces/{workspace_code}")
            }
        }),
    )
    .await;
    let catalog_payload = response_json(catalog_read).await;
    let catalog_text = catalog_payload["result"]["contents"][0]["text"]
        .as_str()
        .expect("catalog resource should contain JSON text");
    let catalog_records: Value =
        serde_json::from_str(catalog_text).expect("catalog text should decode as JSON");
    assert_eq!(catalog_records[0]["key"], "DEFAULT_REGION");
    assert_eq!(catalog_records[0]["value"], "us-east-1");
    assert_eq!(catalog_records[0]["sourceKind"], "Inherited");

    let tools_list = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list"
        }),
    )
    .await;
    let tools_payload = response_json(tools_list).await;
    let tool_names = tools_payload["result"]["tools"]
        .as_array()
        .expect("tool list should be an array")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"variable.upsert_workspace_local"));
    assert!(tool_names.contains(&"variable.delete_workspace_local"));

    let upsert_result = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "variable.upsert_workspace_local",
                "arguments": {
                    "workspaceCode": workspace_code.as_str(),
                    "key": "DEFAULT_REGION",
                    "value": "us-west-2",
                    "description": "Workspace override"
                }
            }
        }),
    )
    .await;
    let upsert_payload = response_json(upsert_result).await;
    assert_eq!(
        upsert_payload["result"]["structuredContent"]["value"],
        "us-west-2"
    );
    assert_eq!(
        upsert_payload["result"]["structuredContent"]["sourceKind"],
        "Override"
    );

    let detail_after_upsert = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "resources/read",
            "params": {
                "uri": format!("tron://variables/workspaces/{workspace_code}/DEFAULT_REGION")
            }
        }),
    )
    .await;
    let detail_after_upsert_payload = response_json(detail_after_upsert).await;
    let detail_after_upsert_text = detail_after_upsert_payload["result"]["contents"][0]["text"]
        .as_str()
        .expect("detail resource should contain JSON text");
    let detail_after_upsert_record: Value =
        serde_json::from_str(detail_after_upsert_text).expect("detail text should decode as JSON");
    assert_eq!(detail_after_upsert_record["value"], "us-west-2");
    assert_eq!(detail_after_upsert_record["sourceKind"], "Override");

    let delete_result = issue_json_request(
        router.clone(),
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "variable.delete_workspace_local",
                "arguments": {
                    "workspaceCode": workspace_code.as_str(),
                    "key": "DEFAULT_REGION"
                }
            }
        }),
    )
    .await;
    let delete_payload = response_json(delete_result).await;
    assert_eq!(
        delete_payload["result"]["structuredContent"]["key"],
        "DEFAULT_REGION"
    );

    let detail_after_delete = issue_json_request(
        router,
        Method::POST,
        "/mcp/variables",
        Some(&access_token),
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "resources/read",
            "params": {
                "uri": format!("tron://variables/workspaces/{workspace_code}/DEFAULT_REGION")
            }
        }),
    )
    .await;
    let detail_after_delete_payload = response_json(detail_after_delete).await;
    let detail_after_delete_text = detail_after_delete_payload["result"]["contents"][0]["text"]
        .as_str()
        .expect("detail resource should contain JSON text");
    let detail_after_delete_record: Value =
        serde_json::from_str(detail_after_delete_text).expect("detail text should decode as JSON");
    assert_eq!(detail_after_delete_record["value"], "us-east-1");
    assert_eq!(detail_after_delete_record["sourceKind"], "Inherited");

    let _ = fs::remove_file(database_path);
}

async fn login(router: &axum::Router) -> String {
    let response = issue_json_request(
        router.clone(),
        Method::POST,
        "/api/v1/auth/login",
        None,
        json!({
            "login": "admin",
            "password": "TronAdminPass123!"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    payload["data"]["accessToken"]
        .as_str()
        .expect("login should return an access token")
        .to_string()
}

async fn enable_gateway(router: &axum::Router, access_token: &str) {
    let response = issue_json_request(
        router.clone(),
        Method::PATCH,
        "/api/v1/mcp/gateway",
        Some(access_token),
        json!({
            "gatewayEnabled": true,
            "variablesEndpointEnabled": true
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

async fn create_workspace(router: &axum::Router, access_token: &str) -> String {
    let response = issue_json_request(
        router.clone(),
        Method::POST,
        "/api/v1/admin/workspaces",
        Some(access_token),
        json!({
            "name": "MCP Variables Workspace",
            "description": "Workspace for MCP variable transport tests",
            "parentWorkspaceCode": null
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    payload["data"]["workspaceCode"]
        .as_str()
        .expect("workspace creation should return a workspace code")
        .to_string()
}

async fn create_platform_variable(router: &axum::Router, access_token: &str) {
    let response = issue_json_request(
        router.clone(),
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(access_token),
        json!({
            "key": "DEFAULT_REGION",
            "value": "us-east-1",
            "description": "Inherited default region"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn issue_json_request(
    router: axum::Router,
    method: Method,
    uri: &str,
    bearer_token: Option<&str>,
    payload: Value,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer_token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }

    router
        .oneshot(
            request
                .body(Body::from(payload.to_string()))
                .expect("request should build"),
        )
        .await
        .expect("request should succeed")
}

async fn response_json(response: Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should decode as JSON")
}

fn test_config(prefix: &str) -> (AppConfig, PathBuf) {
    let database_path = temp_database_path(prefix);
    let mut config = AppConfig::default();
    config.database_url = database_path.display().to_string();
    (config, database_path)
}

fn temp_database_path(prefix: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current system time should be after unix epoch")
        .as_millis();
    std::env::temp_dir().join(format!("{prefix}-{}-{millis}.sqlite3", std::process::id()))
}
