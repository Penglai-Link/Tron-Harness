mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{app, app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn list_variables_returns_effective_catalog_with_inherited_and_override_sources() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-VAR backend RED: effective workspace variable route must exist",
    );

    let payload = response_json(response).await;
    assert_eq!(payload["code"], "SUCCESS");
    let records = payload["data"]
        .as_array()
        .expect("effective variable list should return an array");

    let default_region = records
        .iter()
        .find(|item| item["key"] == "DEFAULT_REGION")
        .expect("child workspace should inherit root variables");
    assert_eq!(default_region["value"], json!("us-east-1"));
    assert_eq!(default_region["sourceKind"], json!("Inherited"));
    assert_eq!(default_region["sourceWorkspaceCode"], json!("ENTERPRISE"));

    let timeout = records
        .iter()
        .find(|item| item["key"] == "ROUTER_TIMEOUT_MS")
        .expect("child workspace should expose the effective timeout value");
    assert_eq!(timeout["value"], json!("400"));
    assert_eq!(timeout["sourceKind"], json!("Override"));
    assert_eq!(timeout["sourceWorkspaceCode"], json!("TRON_CORE"));
    assert_eq!(timeout["shadowedWorkspaceCode"], json!("PLATFORM"));
}

#[tokio::test]
async fn platform_variables_are_inherited_by_workspace_catalogs() {
    let database_url = temp_database_url("variable-platform-inheritance");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let platform_create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key": "GLOBAL_LLM_TIMEOUT_MS",
            "value": "1200",
            "description": "Platform default timeout"
        })),
    )
    .await;
    assert_eq!(platform_create.status(), StatusCode::CREATED);
    let platform_json = response_json(platform_create).await;
    assert_eq!(platform_json["data"]["key"], json!("GLOBAL_LLM_TIMEOUT_MS"));

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let records = list_json["data"]
        .as_array()
        .expect("workspace variable list should return records");
    let inherited = records
        .iter()
        .find(|item| item["key"] == "GLOBAL_LLM_TIMEOUT_MS")
        .expect("workspace should inherit platform variables");
    assert_eq!(inherited["value"], json!("1200"));
    assert_eq!(inherited["sourceKind"], json!("Inherited"));
    assert_eq!(inherited["sourceLabel"], json!("Platform Configuration"));
    assert_eq!(inherited["sourceWorkspaceCode"], json!("PLATFORM"));

    let override_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "GLOBAL_LLM_TIMEOUT_MS",
            "value": "600",
            "description": "Workspace timeout override"
        })),
    )
    .await;
    assert_eq!(override_response.status(), StatusCode::CREATED);
    let override_json = response_json(override_response).await;
    assert_eq!(override_json["data"]["sourceKind"], json!("Override"));
    assert_eq!(
        override_json["data"]["shadowedLabel"],
        json!("Platform Configuration")
    );

    let delete_response = authenticated_request(
        &router,
        Method::DELETE,
        "/api/v1/variables/workspaces/TRON_CORE/GLOBAL_LLM_TIMEOUT_MS",
        None,
    )
    .await;
    assert_eq!(delete_response.status(), StatusCode::OK);

    let fallback_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    let fallback_json = response_json(fallback_response).await;
    let fallback_records = fallback_json["data"]
        .as_array()
        .expect("workspace variable list should remain readable");
    let fallback = fallback_records
        .iter()
        .find(|item| item["key"] == "GLOBAL_LLM_TIMEOUT_MS")
        .expect("deleting workspace override should reveal platform value");
    assert_eq!(fallback["value"], json!("1200"));
    assert_eq!(fallback["sourceKind"], json!("Inherited"));
    assert_eq!(fallback["sourceLabel"], json!("Platform Configuration"));
}

#[tokio::test]
async fn create_override_and_delete_falls_back_to_inherited_value() {
    let database_url = temp_database_url("variable-override-fallback");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "DEFAULT_REGION",
            "value": "ap-southeast-1",
            "description": "Workspace override"
        })),
    )
    .await;
    assert_eq!(
        create_response.status(),
        StatusCode::CREATED,
        "AC-VAR backend RED: creating a local override should succeed",
    );
    let create_json = response_json(create_response).await;
    assert_eq!(create_json["data"]["sourceKind"], json!("Override"));
    assert_eq!(
        create_json["data"]["sourceWorkspaceCode"],
        json!("TRON_CORE")
    );

    let delete_response = authenticated_request(
        &router,
        Method::DELETE,
        "/api/v1/variables/workspaces/TRON_CORE/DEFAULT_REGION",
        None,
    )
    .await;
    assert_eq!(delete_response.status(), StatusCode::OK);

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    let list_json = response_json(list_response).await;
    let records = list_json["data"]
        .as_array()
        .expect("variable list should remain readable after delete");
    let default_region = records
        .iter()
        .find(|item| item["key"] == "DEFAULT_REGION")
        .expect("deleting the override should reveal the inherited ancestor value");
    assert_eq!(default_region["value"], json!("us-east-1"));
    assert_eq!(default_region["sourceKind"], json!("Inherited"));
    assert_eq!(default_region["sourceWorkspaceCode"], json!("ENTERPRISE"));
}

#[tokio::test]
async fn update_local_variable_persists_new_value() {
    let database_url = temp_database_url("variable-update-local");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/variables/workspaces/TRON_CORE/OPENAI_API_KEY",
        Some(json!({
            "value": "sk-tron-core-rotated",
            "description": "Rotated key"
        })),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-VAR backend RED: local workspace variables must be editable",
    );
    let payload = response_json(response).await;
    assert_eq!(payload["data"]["value"], json!("sk-tron-core-rotated"));
    assert_eq!(payload["data"]["sourceKind"], json!("Local"));
}

#[tokio::test]
async fn duplicate_local_keys_and_inherited_updates_are_rejected() {
    let router = app();

    let duplicate_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "OPENAI_API_KEY",
            "value": "duplicate"
        })),
    )
    .await;
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);
    let duplicate_json = response_json(duplicate_response).await;
    assert_eq!(duplicate_json["code"], json!("VARIABLE_KEY_DUPLICATE"));

    let inherited_update_response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/variables/workspaces/TRON_CORE/DEFAULT_REGION",
        Some(json!({
            "value": "eu-central-1"
        })),
    )
    .await;
    assert_eq!(inherited_update_response.status(), StatusCode::BAD_REQUEST);
    let inherited_update_json = response_json(inherited_update_response).await;
    assert_eq!(
        inherited_update_json["code"],
        json!("VARIABLE_INHERITED_MUTATION_INVALID"),
    );
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
