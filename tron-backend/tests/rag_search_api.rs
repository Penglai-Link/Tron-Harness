mod common;

use std::{
    collections::BTreeSet,
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
            "profileId": "memory-related-v1"
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

    let missing_access_workspace = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(search_payload(json!({
            "userCode": "USR-AUTH-0001",
            "agentCode": "AGENT_CODEX"
        }))),
    )
    .await;
    assert_eq!(missing_access_workspace.status(), StatusCode::BAD_REQUEST);
    let missing_access_workspace_json = response_json(missing_access_workspace).await;
    assert_eq!(
        missing_access_workspace_json["code"],
        json!("RAG_SEARCH_REQUEST_INVALID")
    );

    let mut invalid_graph_expansion = search_payload(json!({
        "userCode": "USR-AUTH-0001",
        "agentCode": "AGENT_CODEX",
        "workspaceCode": "TRON_CORE"
    }));
    invalid_graph_expansion["context"]["graphExpansion"] = json!({
        "relationships": ["requires_tool"]
    });
    let invalid_graph_expansion_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(invalid_graph_expansion),
    )
    .await;
    assert_eq!(
        invalid_graph_expansion_response.status(),
        StatusCode::BAD_REQUEST
    );
    let invalid_graph_expansion_json = response_json(invalid_graph_expansion_response).await;
    assert_eq!(
        invalid_graph_expansion_json["code"],
        json!("RAG_SEARCH_REQUEST_INVALID")
    );

    let unknown_workspace = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "memory-related-v1",
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
    assert_eq!(body["data"]["hits"][0]["title"], json!("API memory rule"));
    assert_eq!(body["data"]["hits"][0]["metadata"], json!({}));
    let hit = body["data"]["hits"][0].as_object().unwrap();
    let keys = hit.keys().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "corpusId",
            "sourceCode",
            "sourceVersion",
            "title",
            "score",
            "text",
            "metadata",
        ])
    );
    assert_eq!(body["data"]["contextPack"], json!(null));
}

#[tokio::test]
async fn search_api_flattens_requested_logical_return_fields() {
    let database_url = temp_database_path("rag-search-api-return-fields")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);
    let router = app_with_database(&database_url);
    let mut payload = search_payload(json!({
        "userCode": "USR-AUTH-0001",
        "agentCode": "AGENT_CODEX",
        "workspaceCode": "TRON_CORE"
    }));
    payload["context"]["returnFields"] = json!(["memoryType"]);

    let response =
        authenticated_request(&router, Method::POST, "/api/v1/rag/search", Some(payload)).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["data"]["hits"][0]["memoryType"], json!("RULE"));
    assert!(body["data"]["hits"][0].get("memory_type").is_none());
}

#[tokio::test]
async fn search_api_accepts_missing_query_for_structured_filter_profile() {
    let database_url = temp_database_path("rag-search-api-structured-filter")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);
    insert_structured_filter_profile(&database_url, "memory-api-structured-filter-v1");
    let router = app_with_database(&database_url);
    let mut payload = search_payload(json!({
        "userCode": "USR-AUTH-0001",
        "agentCode": "AGENT_CODEX",
        "workspaceCode": "TRON_CORE"
    }));
    payload["profileId"] = json!("memory-api-structured-filter-v1");
    payload["context"]["returnFields"] = json!(["memoryType"]);
    payload.as_object_mut().unwrap().remove("query");

    let response =
        authenticated_request(&router, Method::POST, "/api/v1/rag/search", Some(payload)).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(
        body["data"]["hits"][0]["sourceCode"],
        json!("api-memory-rule")
    );
    assert_eq!(body["data"]["hits"][0]["memoryType"], json!("RULE"));

    let invalid_filter = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "memory-api-structured-filter-v1",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "filters": { "unknown": "x" },
            "limit": 3,
            "tokenBudget": 100,
            "context": { "strategy": "RESULT_LIST" }
        })),
    )
    .await;
    assert_eq!(invalid_filter.status(), StatusCode::BAD_REQUEST);
    let invalid_filter_body = response_json(invalid_filter).await;
    assert_eq!(
        invalid_filter_body["code"],
        json!("RAG_SEARCH_FILTER_INVALID")
    );
    assert!(invalid_filter_body.get("data").is_some());
}

#[tokio::test]
async fn search_api_rejects_physical_return_field_names() {
    let database_url = temp_database_path("rag-search-api-physical-return-field")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);
    let router = app_with_database(&database_url);
    let mut payload = search_payload(json!({
        "userCode": "USR-AUTH-0001",
        "agentCode": "AGENT_CODEX",
        "workspaceCode": "TRON_CORE"
    }));
    payload["context"]["returnFields"] = json!(["memory_type"]);

    let response =
        authenticated_request(&router, Method::POST, "/api/v1/rag/search", Some(payload)).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("RAG_SEARCH_REQUEST_INVALID"));
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
                text_unit_code, source_code, source_version, scope_type, scope_code,
                source_status, title, source_content_hash, source_metadata, source_created_at,
                source_updated_at, memory_type, memory_status, review_status, version, injectable,
                unit_type, text, heading_path, ordinal, token_count,
                content_hash, status, metadata, created_at, updated_at
            )
            VALUES (?1, ?2, 'v1', 'SPACE', 'TRON_CORE',
                'ACTIVE', 'API memory rule', 'hash', '{}', 1, 1, 'RULE', 'ACTIVE', 'APPROVED', 'v1', 1,
                'LEAF',
                'Always cite sources in API answers.',
                '', 0, 6, 'hash', 'ACTIVE', '{}', 1, 1)",
            params!["api-memory-rule:v1:0", "api-memory-rule"],
        )
        .unwrap();
}

fn insert_structured_filter_profile(database_url: &str, profile_id: &str) {
    let connection = Connection::open(database_url).unwrap();
    let config = json!({
        "routes": [{
            "routeId": "memory-api-structured",
            "retriever": "STRUCTURED_FILTER_SQLITE_V1",
            "filters": {"memoryType": "RULE"}
        }],
        "fusionPolicy": {"type": "SUM_SCORE"},
        "dedupPolicy": {"type": "TEXT_UNIT"},
        "rerankPolicy": {"enabled": false},
        "hydratePolicy": {"type": "TEXT_UNIT"},
        "contextPolicy": {"supportedStrategies": ["RESULT_LIST"]}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'memory-card', '[\"memory-card\"]',
                'v1', ?2, '{}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)",
            params![profile_id, serde_json::to_string(&config).unwrap()],
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
