mod common;
use axum::http::{Method, StatusCode};
use common::{authenticated_request, response_json};
use serde_json::json;
use std::fs;
use tron_backend::config::AppConfig;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn context_layout_exposes_and_saves_host_budgets_with_the_renamed_summary() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("application.yaml");
    fs::write(&path, "port: 8080\n").unwrap();
    let app = common::app_with_config(AppConfig {
        database_url: root.path().join("tron.sqlite3").display().to_string(),
        graph_storage_path: root.path().join("graph.grafeo").display().to_string(),
        skills_registry_root: root.path().join("skills").display().to_string(),
        ..AppConfig::from_yaml_file(&path).unwrap()
    });
    let response = authenticated_request(&app, Method::GET, "/api/v1/context-layout", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let document = response_json(response).await["data"].clone();
    assert!(document["order"].as_array().unwrap().contains(&json!("conversation_history_summary")));
    assert_eq!(document["nativeBudgets"]["conversation_history_summary"]["maxTokens"], 6000);
    assert!(document["nativeBudgets"].get("seed_prompt_prefix").is_none());
    assert!(document["nativeBudgets"].get("current_session_history").is_none());
    assert!(document["nativeBudgets"].get("skills.catalog").is_none());
    let mut requested = document.clone();
    requested.as_object_mut().unwrap().remove("blocks");
    requested["nativeBudgets"]["conversation_history_summary"] = json!({"maxTokens":4000,"maxRatioBps":1500});
    requested["nativeBudgets"]["page_context"]["maxTokens"] = json!(3000);
    let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(requested)).await;
    assert_eq!(response.status(), StatusCode::OK, "{}", response_json(response).await);
    let loaded = AppConfig::from_yaml_file(&path).unwrap().short_term_memory;
    assert_eq!(loaded.context.blocks.conversation_history_summary.max_tokens, Some(4000));
    assert_eq!(loaded.context.blocks.conversation_history_summary.max_ratio_bps, Some(1500));
    assert_eq!(loaded.context.providers.data_blocks["page_context"].max_tokens, Some(3000));
    let response = authenticated_request(&app, Method::GET, "/api/v1/context-layout", None).await;
    let mut invalid = response_json(response).await["data"].clone();
    invalid.as_object_mut().unwrap().remove("blocks");
    invalid["nativeBudgets"]["seed_prompt_prefix"] = json!({"maxTokens":1,"maxRatioBps":null});
    let before = fs::read(&path).unwrap();
    let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(invalid)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(fs::read(&path).unwrap(), before);
}
