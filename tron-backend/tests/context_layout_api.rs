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
    let response = authenticated_request(&app, Method::GET, "/api/v1/context-blocks", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let catalog = response_json(response).await["data"].clone();
    assert!(catalog.as_array().unwrap().iter().all(|block| matches!(block["updateMode"].as_str(), Some("Fixed" | "Append" | "Replace"))), "every shipped source must declare its update mode");
    for block in document["blocks"].as_array().unwrap() {
        assert!(!block["description"].as_str().unwrap().is_empty(), "{}", block["id"]);
        let metadata = catalog.as_array().unwrap().iter().find(|item| item["id"] == block["id"]).unwrap();
        assert_eq!(block["displayName"], metadata["displayName"]);
        assert_eq!(block["description"], metadata["description"]);
        assert_eq!(block["updateMode"], metadata["updateMode"]);
    }
    let skills = catalog.as_array().unwrap().iter().find(|item| item["id"] == "skills.catalog").unwrap();
    assert_eq!(skills["displayName"], "技能目录");
    assert!(skills["providerId"].is_string());
    assert!(skills["description"].as_str().unwrap().contains("已授权技能"));
    assert_eq!(skills["updateMode"], "Replace");
    for (id, mode) in [("seed_prompt_prefix", "Fixed"), ("current_session_history", "Append"), ("conversation_history_summary", "Replace"), ("page_context", "Replace"), ("application_context", "Replace")] {
        assert_eq!(catalog.as_array().unwrap().iter().find(|item| item["id"] == id).unwrap()["updateMode"], mode);
    }
    use tower::ServiceExt;
    let response = app.clone().oneshot(axum::http::Request::builder()
        .uri("/api/v1/context-blocks")
        .header("authorization", "Bearer profile-test-token")
        .header("accept-language", "en-US")
        .body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let english = response_json(response).await["data"].clone();
    assert_eq!(english.as_array().unwrap().iter().find(|item| item["id"] == "skills.catalog").unwrap()["displayName"], "Skill catalog");
    assert!(english.as_array().unwrap().iter().find(|item| item["id"] == "application_context").unwrap()["description"].as_str().unwrap().contains("AGENTS.md"));
    let defaults = document["defaults"].clone();
    assert_eq!(defaults["shares"]["seed_prompt_prefix"], 500);
    assert_eq!(defaults["shares"]["current_session_history"], 7500);
    let mut requested = document.clone();
    requested.as_object_mut().unwrap().remove("blocks");
    requested.as_object_mut().unwrap().remove("defaults");
    requested["shares"]["conversation_history_summary"] = json!(300);
    requested["shares"]["current_session_history"] = json!(7600);
    let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(requested)).await;
    assert_eq!(response.status(), StatusCode::OK, "{}", response_json(response).await);
    let loaded = AppConfig::from_yaml_file(&path).unwrap().short_term_memory;
    assert_eq!(loaded.context.providers.shares["conversation_history_summary"], 300);
    assert_eq!(loaded.model_context_budget_plan(Some(1_000_000), Some(0)).block_budgets.conversation_history_summary.effective_limit_tokens, 30_000);
    let response = authenticated_request(&app, Method::GET, "/api/v1/context-layout", None).await;
    let mut invalid = response_json(response).await["data"].clone();
    assert_eq!(invalid["defaults"], defaults, "saved overrides must not change reset defaults");
    invalid.as_object_mut().unwrap().remove("blocks");
    invalid.as_object_mut().unwrap().remove("defaults");
    let before = fs::read(&path).unwrap();
    for share in [299, 301] {
        invalid["shares"]["conversation_history_summary"] = json!(share);
        let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(invalid.clone())).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    let response = authenticated_request(&app, Method::PUT, "/api/v1/context-layout", Some(json!({
        "revision": invalid["revision"], "order": defaults["order"], "shares": defaults["shares"],
    }))).await;
    assert_eq!(response.status(), StatusCode::OK, "{}", response_json(response).await);
    let loaded = AppConfig::from_yaml_file(&path).unwrap().short_term_memory;
    let built_in = tron_backend::application::short_term_memory::ContextProvidersConfig::default();
    assert_eq!(loaded.context.providers.order, built_in.order);
    assert_eq!(loaded.context.providers.shares, built_in.shares);

}
