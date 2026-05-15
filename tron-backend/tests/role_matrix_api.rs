use std::path::PathBuf;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn list_permission_domains_returns_supported_matrix_headers() {
    let response = request(
        AppConfig::default(),
        Method::GET,
        "/api/v1/admin/permission-domains",
        None,
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-RM backend RED: permission-domain endpoint for role matrix must exist",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert_eq!(
        json["data"],
        json!(["Workspace", "Models", "Tasks", "Memory", "Skills", "MCP"]),
    );
}

#[tokio::test]
async fn list_roles_returns_seeded_catalog_with_codes_and_permissions() {
    let response = request(
        AppConfig::default(),
        Method::GET,
        "/api/v1/admin/roles",
        None,
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-RM backend RED: role catalog read endpoint must exist",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    let roles = json["data"]
        .as_array()
        .expect("roles data should be an array");
    assert_eq!(roles.len(), 4);
    assert!(roles.iter().any(|role| role["code"] == "ADMIN"));
    assert!(roles.iter().any(|role| role["code"] == "WORKSPACE_ADMIN"));
    assert!(roles.iter().all(|role| role["permissions"].is_object()));
}

#[tokio::test]
async fn replace_roles_persists_catalog_to_configured_sqlite_database() {
    let database_url = temp_database_url("role-matrix-catalog");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };

    let payload = json!({
        "roles": [
            {
                "id": "734300000000000001",
                "code": "ADMIN",
                "name": "Admin",
                "description": "Full System Access",
                "kind": "System",
                "managedBy": "Platform",
                "updatedAt": "2026-05-15T00:00:00Z",
                "permissions": {
                    "Workspace": ["R", "W", "D", "A"],
                    "Models": ["R", "W", "D", "A"],
                    "Tasks": ["R", "W", "D", "A"],
                    "Memory": ["R", "W", "D", "A"],
                    "Skills": ["R", "W", "D", "A"],
                    "MCP": ["R", "W", "D", "A"]
                }
            },
            {
                "id": "734300000000000002",
                "code": "WORKSPACE_ADMIN",
                "name": "Workspace Admin",
                "description": "Workspace administration",
                "kind": "System",
                "managedBy": "Workspace",
                "updatedAt": "2026-05-15T00:00:00Z",
                "permissions": {
                    "Workspace": ["R", "W"],
                    "Models": ["R", "W"],
                    "Tasks": ["R", "W"],
                    "Memory": ["R"],
                    "Skills": ["R", "W"],
                    "MCP": ["R"]
                }
            },
            {
                "id": "734300000000000003",
                "code": "DEVELOPER",
                "name": "Developer",
                "description": "Build and operate applications",
                "kind": "System",
                "managedBy": "Platform",
                "updatedAt": "2026-05-15T00:00:00Z",
                "permissions": {
                    "Workspace": ["R"],
                    "Models": ["R", "W"],
                    "Tasks": ["R", "W"],
                    "Memory": ["R"],
                    "Skills": ["R", "W"],
                    "MCP": ["R"]
                }
            },
            {
                "id": "734300000000000004",
                "code": "VIEWER",
                "name": "Viewer",
                "description": "Read-only Access",
                "kind": "System",
                "managedBy": "Platform",
                "updatedAt": "2026-05-15T00:00:00Z",
                "permissions": {
                    "Workspace": ["R"],
                    "Models": ["R"],
                    "Tasks": ["R"],
                    "Memory": ["R"],
                    "Skills": ["R"],
                    "MCP": ["R"]
                }
            },
            {
                "code": "CUSTOM_RELEASE_MANAGER",
                "name": "Release Manager",
                "description": "Coordinates release approval windows.",
                "kind": "Custom",
                "managedBy": "Workspace",
                "updatedAt": "2026-05-15T00:00:00Z",
                "permissions": {
                    "Workspace": ["R"],
                    "Models": ["R", "W"],
                    "Tasks": ["R", "W", "A"],
                    "Memory": [],
                    "Skills": ["R", "W"],
                    "MCP": ["R"]
                }
            }
        ]
    });

    let replace_response = request(
        config.clone(),
        Method::PUT,
        "/api/v1/admin/roles",
        Some(payload),
    )
    .await;

    assert_eq!(
        replace_response.status(),
        StatusCode::OK,
        "AC-RM backend RED: role catalog save endpoint must persist the supplied snapshot",
    );

    let replace_json = response_json(replace_response).await;
    assert_eq!(replace_json["code"], "SUCCESS");
    assert!(
        replace_json["data"]
            .as_array()
            .expect("replace response should return roles")
            .iter()
            .any(|role| role["code"] == "CUSTOM_RELEASE_MANAGER"),
    );

    let reloaded_response = request(config, Method::GET, "/api/v1/admin/roles", None).await;
    let reloaded_json = response_json(reloaded_response).await;
    let reloaded_roles = reloaded_json["data"]
        .as_array()
        .expect("roles data should be an array");
    assert!(
        reloaded_roles
            .iter()
            .any(|role| role["code"] == "CUSTOM_RELEASE_MANAGER"),
        "AC-RM backend GREEN: saved custom role should survive a fresh router against the same SQLite database",
    );
}

async fn request(
    config: AppConfig,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let router = build_router(config);
    let mut builder = Request::builder().method(method).uri(uri);
    let request_body = if let Some(json_body) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    router
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
