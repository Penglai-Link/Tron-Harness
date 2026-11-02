mod common;
use axum::http::{Method, StatusCode};
use common::{authenticated_request, response_json};
use serde_json::json;
use std::fs;
use tron_backend::config::AppConfig;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn context_layout_saves_only_shares_and_requires_exactly_one_hundred_percent() {
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
    assert!(document.get("nativeBudgets").is_none());
    let mut requested = document.clone();
    requested.as_object_mut().unwrap().remove("blocks");
    requested["shares"]["conversation_history_summary"] = json!(300);
    requested["shares"]["current_session_history"] = json!(7600);
    let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(requested)).await;
    assert_eq!(response.status(), StatusCode::OK, "{}", response_json(response).await);
    let loaded = AppConfig::from_yaml_file(&path).unwrap().short_term_memory;
    assert_eq!(loaded.context.providers.shares["conversation_history_summary"], 300);
    assert_eq!(loaded.model_context_budget_plan(Some(1_000_000), Some(0)).block_budgets.conversation_history_summary.effective_limit_tokens, 30_000);
    let response = authenticated_request(&app, Method::GET, "/api/v1/context-layout", None).await;
    let mut invalid = response_json(response).await["data"].clone();
    invalid.as_object_mut().unwrap().remove("blocks");
    let before = fs::read(&path).unwrap();
    for share in [299, 301] {
        invalid["shares"]["conversation_history_summary"] = json!(share);
        let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(invalid.clone())).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}
