mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use common::{app, authenticated_request, response_json};

#[tokio::test]
async fn ingest_api_validates_required_request_and_direct_target_scope() {
    let router = app();

    let malformed = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/ingest",
        Some(json!({
            "corpusId": "memory-card",
            "ingestProfileId": "memory-card-ingest-v1",
            "workspaceCode": "TRON_CORE"
        })),
    )
    .await;
    assert_eq!(
        malformed.status(),
        StatusCode::BAD_REQUEST,
        "AC-RID-01 RED: malformed ingest request must be rejected by the route"
    );
    let malformed_json = response_json(malformed).await;
    assert_eq!(malformed_json["code"], json!("RAG_INGEST_REQUEST_INVALID"));
    assert!(malformed_json.get("data").is_some());

    let missing_agent_code = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/ingest",
        Some(json!({
            "corpusId": "memory-card",
            "ingestProfileId": "memory-card-ingest-v1",
            "workspaceCode": "TRON_CORE",
            "accessContext": {
                "userCode": "USR-LARGE-ID",
                "workspaceCode": "TRON_CORE"
            },
            "targetScope": {
                "scopeType": "SPACE",
                "scopeCode": "TRON_CORE"
            },
            "sourceCode": "memory-rag-rule",
            "sourceVersion": "v1",
            "sourceType": "PLAIN_TEXT",
            "payload": {
                "title": "RAG citation rule",
                "text": "Always cite source titles."
            }
        })),
    )
    .await;
    assert_eq!(missing_agent_code.status(), StatusCode::BAD_REQUEST);
    let missing_agent_json = response_json(missing_agent_code).await;
    assert_eq!(
        missing_agent_json["code"],
        json!("RAG_INGEST_REQUEST_INVALID")
    );

    let cross_user_scope = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/ingest",
        Some(json!({
            "corpusId": "memory-card",
            "ingestProfileId": "memory-card-ingest-v1",
            "workspaceCode": "TRON_CORE",
            "accessContext": {
                "userCode": "USR-LARGE-ID",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "TRON_CORE"
            },
            "targetScope": {
                "scopeType": "USER",
                "scopeCode": "OTHER-USER"
            },
            "sourceCode": "memory-rag-rule",
            "sourceVersion": "v1",
            "sourceType": "PLAIN_TEXT",
            "payload": {
                "title": "RAG citation rule",
                "text": "Always cite source titles."
            }
        })),
    )
    .await;
    assert_eq!(cross_user_scope.status(), StatusCode::FORBIDDEN);
    let cross_user_json = response_json(cross_user_scope).await;
    assert_eq!(cross_user_json["code"], json!("RAG_INGEST_SCOPE_INVALID"));
    assert!(cross_user_json.get("message").is_some());
}

#[tokio::test]
async fn ingest_api_requires_authenticated_workspace_access() {
    let router = app();

    let unknown_workspace = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/rag/ingest",
        Some(json!({
            "corpusId": "project-doc",
            "ingestProfileId": "project-doc-ingest-v1",
            "workspaceCode": "UNKNOWN_WORKSPACE",
            "accessContext": {
                "userCode": "USR-LARGE-ID",
                "agentCode": "AGENT_CODEX",
                "workspaceCode": "UNKNOWN_WORKSPACE"
            },
            "targetScope": {
                "scopeType": "SPACE",
                "scopeCode": "UNKNOWN_WORKSPACE"
            },
            "sourceCode": "doc-rag-mvp",
            "sourceVersion": "v1",
            "sourceType": "MARKDOWN",
            "payload": {
                "title": "RAG MVP",
                "text": "# RAG MVP"
            }
        })),
    )
    .await;

    assert_eq!(unknown_workspace.status(), StatusCode::FORBIDDEN);
    let body = response_json(unknown_workspace).await;
    assert_eq!(body["code"], json!("RAG_INGEST_SCOPE_INVALID"));
}
