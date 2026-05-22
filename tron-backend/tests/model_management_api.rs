mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{app, app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn list_workspace_models_inherit_enabled_platform_models() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "SUCCESS");
    assert_eq!(payload["data"], json!([
        {
            "modelCode": "claude-3-opus-20240229",
            "name": "claude-3-opus-20240229",
            "purpose": "Complex Tasks",
            "vendor": "Anthropic",
            "context": "200k tokens",
            "maxOutput": "4,096 tokens",
            "trainingData": "Aug 2023",
            "speed": "~31 tk/s",
            "price": "$15.00 / 1M Input",
            "status": "Available",
            "sourceWorkspaceCode": "PLATFORM",
            "sourceWorkspaceName": "Platform",
            "sourceLabel": "Platform",
            "removedAt": null
        },
        {
            "modelCode": "gemini-1.5-pro",
            "name": "gemini-1.5-pro",
            "purpose": "Multimodal Review",
            "vendor": "Google",
            "context": "2M tokens",
            "maxOutput": "8,192 tokens",
            "trainingData": "Apr 2024",
            "speed": "~36 tk/s",
            "price": "$7.00 / 1M Input",
            "status": "Available",
            "sourceWorkspaceCode": "PLATFORM",
            "sourceWorkspaceName": "Platform",
            "sourceLabel": "Platform",
            "removedAt": null
        },
        {
            "modelCode": "gpt-4-turbo",
            "name": "gpt-4-turbo",
            "purpose": "Default Reasoning",
            "vendor": "OpenAI",
            "context": "128k tokens",
            "maxOutput": "4,096 tokens",
            "trainingData": "Dec 2023",
            "speed": "~42 tk/s",
            "price": "$10.00 / 1M Input",
            "status": "Available",
            "sourceWorkspaceCode": "PLATFORM",
            "sourceWorkspaceName": "Platform",
            "sourceLabel": "Platform",
            "removedAt": null
        }
    ]));
}

#[tokio::test]
async fn remove_and_restore_only_affect_current_workspace_overlay() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("model-management-flow")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let removed = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE/gpt-4-turbo/remove",
        None,
    )
    .await;
    assert_eq!(removed.status(), StatusCode::OK);
    assert_eq!(
        response_json(removed).await["data"],
        json!({
            "workspaceCode": "TRON_CORE",
            "modelCode": "gpt-4-turbo",
            "status": "Removed",
            "removedAt": "2026-05-22T00:00:00Z"
        })
    );

    let tron_core = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(tron_core.status(), StatusCode::OK);
    assert_eq!(
        response_json(tron_core).await["data"][2]["status"],
        "Removed"
    );

    let platform = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/PLATFORM",
        None,
    )
    .await;
    assert_eq!(platform.status(), StatusCode::OK);
    assert_eq!(
        response_json(platform).await["data"][2]["status"],
        "Available"
    );

    let compliance = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/COMPLIANCE",
        None,
    )
    .await;
    assert_eq!(compliance.status(), StatusCode::OK);
    assert_eq!(
        response_json(compliance).await["data"][2]["status"],
        "Available"
    );

    let restored = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE/gpt-4-turbo/restore",
        None,
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(
        response_json(restored).await["data"],
        json!({
            "workspaceCode": "TRON_CORE",
            "modelCode": "gpt-4-turbo",
            "status": "Available",
            "removedAt": null
        })
    );
}

#[tokio::test]
async fn platform_workspace_cannot_remove_inherited_source_model() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/PLATFORM/gpt-4-turbo/remove",
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await["code"],
        "MODEL_PLATFORM_MUTATION_DENIED"
    );
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
