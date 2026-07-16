mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{app, app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn list_permission_domains_returns_supported_matrix_headers() {
    let router = app();
    let response = authenticated_request(
        &router,
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
        json!([
            "Workspace",
            "Models",
            "Tasks",
            "Memory",
            "Skills",
            "MCP",
            "Resources"
        ]),
    );
}

#[tokio::test]
async fn list_roles_returns_seeded_catalog_with_codes_and_permissions() {
    let router = app();
    let response = authenticated_request(&router, Method::GET, "/api/v1/admin/roles", None).await;

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
    let router = app_with_config(config.clone());

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
                    "MCP": ["R", "W", "D", "A"],
                    "Resources": ["R", "W", "D", "A"]
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
                    "MCP": ["R"],
                    "Resources": ["R", "W"]
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
                    "MCP": ["R"],
                    "Resources": ["R", "W"]
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
                    "MCP": ["R"],
                    "Resources": ["R"]
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
                    "MCP": ["R"],
                    "Resources": []
                }
            }
        ]
    });

    let replace_response =
        authenticated_request(&router, Method::PUT, "/api/v1/admin/roles", Some(payload)).await;

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

    let reloaded_router = app_with_config(config);
    let reloaded_response =
        authenticated_request(&reloaded_router, Method::GET, "/api/v1/admin/roles", None).await;
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
    let custom_role = reloaded_roles
        .iter()
        .find(|role| role["code"] == "CUSTOM_RELEASE_MANAGER")
        .expect("custom role should remain available after migration backfill");
    assert!(
        custom_role["permissions"].get("Resources").is_none(),
        "resource permission backfill must not rewrite custom roles",
    );
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
