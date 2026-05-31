mod common;

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use rusqlite::Connection;
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository, config::AppConfig,
};

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn debug_api_validates_workspace_scope_and_returns_uniform_envelope() {
    let database_url = temp_database_path("rag-debug-api")
        .to_string_lossy()
        .to_string();
    seed_source_fixture(&database_url);
    let router = app_with_database(&database_url);

    let malformed = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/debug/source?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(malformed).await["code"],
        json!("RAG_DEBUG_REQUEST_INVALID")
    );

    let cross_scope = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/debug/source?workspaceCode=TRON_CORE&corpusId=project-doc&sourceCode=doc-api&sourceVersion=v1&scopeType=SPACE&scopeCode=OTHER",
        None,
    )
    .await;
    assert_eq!(cross_scope.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_json(cross_scope).await["code"],
        json!("RAG_DEBUG_SCOPE_INVALID")
    );

    let ok = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/rag/debug/source?workspaceCode=TRON_CORE&corpusId=project-doc&sourceCode=doc-api&sourceVersion=v1&scopeType=SPACE&scopeCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(ok.status(), StatusCode::OK);
    let body = response_json(ok).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(body["data"]["source"]["sourceCode"], json!("doc-api"));
    assert_eq!(
        body["data"]["textUnits"][0]["textUnitCode"],
        json!("tu-api-1")
    );
    assert!(body["data"]["textUnits"][0].get("embedding").is_none());
}

#[tokio::test]
async fn retry_api_rejects_unsupported_stage_and_rebuilds_indexes() {
    let database_url = temp_database_path("rag-retry-api")
        .to_string_lossy()
        .to_string();
    seed_source_fixture(&database_url);
    let router = app_with_database(&database_url);

    let bad_stage = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/retry/index",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "corpusId": "project-doc",
            "sourceCode": "doc-api",
            "sourceVersion": "v1",
            "scopeType": "SPACE",
            "scopeCode": "TRON_CORE",
            "stages": ["parse"]
        })),
    )
    .await;
    assert_eq!(bad_stage.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(bad_stage).await["code"],
        json!("RAG_RETRY_STAGE_UNSUPPORTED")
    );

    let ok = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/retry/index",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "corpusId": "project-doc",
            "sourceCode": "doc-api",
            "sourceVersion": "v1",
            "scopeType": "SPACE",
            "scopeCode": "TRON_CORE",
            "stages": ["fulltextIndex"]
        })),
    )
    .await;
    assert_eq!(ok.status(), StatusCode::OK);
    let body = response_json(ok).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(
        body["data"]["stageResults"][0]["stageId"],
        json!("fulltextIndex")
    );
    assert_eq!(
        query_string(
            &Connection::open(&database_url).unwrap(),
            "SELECT fulltext_status FROM rag_project_doc_text_units WHERE source_code = 'doc-api'"
        ),
        "INDEXED"
    );
}

#[tokio::test]
async fn search_api_writes_trace_only_when_enabled() {
    let database_url = temp_database_path("rag-search-trace-api")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);
    let router = app_with_database(&database_url);

    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/search",
        Some(json!({
            "profileId": "memory-related-v1",
            "workspaceCode": "TRON_CORE",
            "query": "cite sources",
            "accessContext": {
                "userCode": "USR-AUTH-0001",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "filters": { "memoryType": "RULE" },
            "limit": 3,
            "tokenBudget": 100,
            "context": { "strategy": "RESULT_LIST" },
            "trace": { "enabled": true }
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert!(body["data"]["traceCode"].as_str().is_some());
    assert_eq!(
        query_i64(
            &Connection::open(&database_url).unwrap(),
            "SELECT COUNT(*) FROM rag_diagnostics WHERE record_type = 'SEARCH_TRACE'"
        ),
        1
    );
}

fn app_with_database(database_url: &str) -> axum::Router {
    let config = AppConfig {
        database_url: database_url.into(),
        ..AppConfig::default()
    };
    app_with_config(config)
}

fn seed_source_fixture(database_url: &str) {
    SqliteRagRepository::connect(database_url).expect("RAG schema should initialize");
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_sources (
                source_code, source_version, source_type, scope_type, scope_code, status,
                title, content_hash, document_code, document_type, path, body_hash, metadata,
                created_at, updated_at, repo_code
            )
            VALUES ('doc-api', 'v1', 'MARKDOWN', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'API Debug Doc', 'hash', 'DOC_API', 'DESIGN', 'docs/api.md', 'body-hash',
                '{}', 1, 1, 'REPO_TRON')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_text_units (
                text_unit_code, source_code, source_version, document_code, scope_type, scope_code,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                content_hash, status, fulltext_status, embedding_status, metadata, created_at, updated_at
            )
            VALUES ('tu-api-1', 'doc-api', 'v1', 'DOC_API', 'SPACE', 'TRON_CORE', 'LEAF',
                'Retry API text.', '', 'API', 0, 3, 'hash', 'ACTIVE', 'FAILED',
                'FAILED', '{}', 1, 1)",
            [],
        )
        .unwrap();
}

fn seed_search_fixture(database_url: &str) {
    SqliteRagRepository::connect(database_url).expect("RAG schema should initialize");
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_memory_sources (
                source_code, source_version, source_type, scope_type, scope_code, status,
                title, content_hash, metadata, created_at, updated_at, memory_type,
                memory_status, review_status, version, injectable
            )
            VALUES ('memory-trace-api', 'v1', 'PLAIN_TEXT', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'Trace memory rule', 'hash', '{}', 1, 1, 'RULE', 'ACTIVE', 'APPROVED', 'v1', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_memory_text_units (
                text_unit_code, source_code, source_version, scope_type, scope_code,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                content_hash, status, fulltext_status, embedding_status, metadata, created_at, updated_at
            )
            VALUES ('tu-trace-api-1', 'memory-trace-api', 'v1', 'SPACE', 'TRON_CORE', 'LEAF',
                'Always cite sources in API answers.', 'Always cite sources in API answers.',
                '', 0, 6, 'hash', 'ACTIVE', 'INDEXED', 'FAILED', '{}', 1, 1)",
            [],
        )
        .unwrap();
}

fn query_string(connection: &Connection, sql: &str) -> String {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn query_i64(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
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
