mod common;

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use rusqlite::{Connection, params};
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository, config::AppConfig,
};

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn search_api_validates_request_auth_workspace_and_scope() {
    let database_url = temp_database_path("rag-search-api-validation")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let malformed = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "memory-related-v1",
            "workspaceCode": "TRON_CORE"
        })),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    let malformed_json = response_json(malformed).await;
    assert_eq!(malformed_json["code"], json!("RAG_SEARCH_REQUEST_INVALID"));
    assert!(malformed_json.get("data").is_some());

    let cross_user_scope = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(search_payload(json!({
            "userCode": "OTHER-USER",
            "agentCode": "AGENT_CODEX",
            "workspaceCode": "TRON_CORE"
        }))),
    )
    .await;
    assert_eq!(cross_user_scope.status(), StatusCode::FORBIDDEN);
    let cross_user_json = response_json(cross_user_scope).await;
    assert_eq!(cross_user_json["code"], json!("RAG_SEARCH_SCOPE_INVALID"));

    let unknown_workspace = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "memory-related-v1",
            "workspaceCode": "UNKNOWN_WORKSPACE",
            "query": "cite sources",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "UNKNOWN_WORKSPACE"
            },
            "filters": {},
            "limit": 3,
            "tokenBudget": 100,
            "context": { "strategy": "RESULT_LIST" }
        })),
    )
    .await;
    assert_eq!(unknown_workspace.status(), StatusCode::FORBIDDEN);
    let unknown_workspace_json = response_json(unknown_workspace).await;
    assert_eq!(
        unknown_workspace_json["code"],
        json!("RAG_SEARCH_SCOPE_INVALID")
    );
}

#[tokio::test]
async fn search_api_returns_uniform_success_envelope_for_result_list() {
    let database_url = temp_database_path("rag-search-api-success")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);
    let router = app_with_database(&database_url);

    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(search_payload(json!({
            "userCode": "USR-AUTH-0001",
            "agentCode": "AGENT_CODEX",
            "workspaceCode": "TRON_CORE"
        }))),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert!(body["message"].is_string());
    assert_eq!(body["data"]["profileId"], json!("memory-related-v1"));
    assert_eq!(body["data"]["contextStrategy"], json!("RESULT_LIST"));
    assert_eq!(
        body["data"]["hits"][0]["sourceCode"],
        json!("api-memory-rule")
    );
    assert_eq!(body["data"]["contextPack"], json!(null));
}

#[tokio::test]
async fn search_api_uses_retrieval_specific_missing_profile_error() {
    let database_url = temp_database_path("rag-search-api-missing-profile")
        .to_string_lossy()
        .to_string();
    let router = app_with_database(&database_url);

    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "missing-retrieval-profile",
            "workspaceCode": "TRON_CORE",
            "query": "cite sources",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "filters": {},
            "limit": 3,
            "tokenBudget": 100,
            "context": { "strategy": "RESULT_LIST" }
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("RAG_RETRIEVAL_PROFILE_NOT_FOUND"));
}

fn app_with_database(database_url: &str) -> axum::Router {
    let config = AppConfig {
        database_url: database_url.into(),
        ..AppConfig::default()
    };
    app_with_config(config)
}

fn search_payload(access_context: serde_json::Value) -> serde_json::Value {
    json!({
        "profileId": "memory-related-v1",
        "workspaceCode": "TRON_CORE",
        "query": "cite sources",
        "accessContext": access_context,
        "filters": { "memoryType": "RULE" },
        "limit": 3,
        "tokenBudget": 100,
        "context": { "strategy": "RESULT_LIST" }
    })
}

fn seed_search_fixture(database_url: &str) {
    SqliteRagRepository::connect(database_url).expect("RAG schema should initialize");
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_memory_text_units (
                text_unit_code, source_code, source_version, source_type, scope_type, scope_code,
                source_status, title, source_content_hash, source_metadata, source_created_at,
                source_updated_at, memory_type, memory_status, review_status, version, injectable,
                unit_type, text, heading_path, ordinal, token_count,
                content_hash, status, metadata, created_at, updated_at
            )
            VALUES (?1, ?2, 'v1', 'PLAIN_TEXT', 'SPACE', 'TRON_CORE',
                'ACTIVE', 'API memory rule', 'hash', '{}', 1, 1, 'RULE', 'ACTIVE', 'APPROVED', 'v1', 1,
                'LEAF',
                'Always cite sources in API answers.',
                '', 0, 6, 'hash', 'ACTIVE', '{}', 1, 1)",
            params!["api-memory-rule:v1:0", "api-memory-rule"],
        )
        .unwrap();
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-{test_name}-{nanos}-{}.sqlite3",
        std::process::id()
    ))
}
