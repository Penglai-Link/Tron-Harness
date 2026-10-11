//! Platform chat transport authorizes persisted ownership without a workspace alias.
mod common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::config::{AppConfig, ResourceConfig};

async fn scoped(
    router: &Router,
    method: Method,
    uri: &str,
    dude: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(dude) = dude {
        builder = builder.header("x-dude-code", dude);
    }
    let body = match body {
        Some(body) => {
            builder = builder.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn platform_chat_reads_require_persisted_dude_and_profile_ownership() {
    let directory = tempfile::tempdir().unwrap();
    let config = common::prepared_config(AppConfig {
        database_url: directory.path().join("scope.sqlite3").display().to_string(),
        graph_storage_path: directory.path().join("scope.grafeo").display().to_string(),
        skills_registry_root: directory.path().join("skills").display().to_string(),
        resource: ResourceConfig::with_default_root(
            directory.path().join("resources").display().to_string(),
        ),
        ..AppConfig::default()
    });
    let mut captured = None;
    let router = common::app_with_state_setup(config, |state| captured = Some(state.clone()));
    let state = captured.unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    let response =
        common::authenticated_request(&router, Method::POST, "/api/v1/dudes/main/session", None)
            .await;
    assert_eq!(response.status(), StatusCode::OK);
    let resolved = common::response_json(response).await;
    let session = resolved["data"]["sessionCode"].as_str().unwrap();
    let dude = resolved["data"]["ownerDudeCode"].as_str().unwrap();
    assert!(resolved["data"]["workspaceCode"].is_null());
    let other = common::authenticated_request(&router, Method::GET, "/api/v1/dudes", None).await;
    let other = common::response_json(other).await;
    let other_dude = other["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["dudeCode"].as_str() != Some(dude))
        .unwrap()["dudeCode"]
        .as_str()
        .unwrap();
    // The product endpoint already creates the real native session. Verify that
    // projection directly, without introducing a second session or fake workspace.
    let framework = state.agent_framework.storage_framework().await.unwrap();
    let native = framework.sessions().unwrap().get(session).await.unwrap();
    assert_eq!(native.id, session);
    let reads = [
        (
            Method::POST,
            format!("/api/v1/chats/sessions/{session}/history/query"),
            Some(json!({"size":20,"cursor":null})),
        ),
        (
            Method::GET,
            format!("/api/v1/chats/sessions/{session}/a2ui-state"),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/agents/runs?conversationCode={session}"),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/agents/agui/streams/{session}/protocol-events"),
            None,
        ),
    ];
    for (method, uri, body) in &reads {
        let response = scoped(&router, method.clone(), uri, Some(dude), body.clone()).await;
        let status = response.status();
        let payload = common::response_json(response).await;
        assert_eq!(status, StatusCode::OK, "platform read {uri}: {payload}");
        let response = scoped(&router, method.clone(), uri, None, body.clone()).await;
        assert!(
            !response.status().is_success(),
            "Dude history requires explicit scope: {uri}"
        );
        let response = scoped(&router, method.clone(), uri, Some(other_dude), body.clone()).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "foreign owner: {uri}"
        );
    }
    let ordinary = common::authenticated_request(
        &router,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({"workspaceCode":"TRON_CORE","title":"Ordinary user session"})),
    )
    .await;
    assert_eq!(ordinary.status(), StatusCode::CREATED);
    let ordinary = common::response_json(ordinary).await;
    let ordinary = ordinary["data"]["sessionCode"].as_str().unwrap();
    for (method, suffix, body) in [
        (
            Method::POST,
            "history/query",
            Some(json!({"size":20,"cursor":null})),
        ),
        (Method::GET, "a2ui-state", None),
        (
            Method::POST,
            "messages/stream",
            Some(json!({"content":"must not run","workspaceCode":null})),
        ),
    ] {
        let uri = format!("/api/v1/chats/sessions/{ordinary}/{suffix}?sessionCode={session}");
        let response = scoped(&router, method, &uri, Some(dude), body).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "query must not substitute path ownership: {uri}"
        );
    }
    state
        .profiles
        .register_or_update("USR-FOREIGN-PROFILE", "another.operator")
        .unwrap();
    for (method, uri, body) in reads.into_iter().chain(std::iter::once((
        Method::POST,
        format!("/api/v1/chats/sessions/{session}/messages/stream"),
        Some(json!({"content":"must not run","workspaceCode":null})),
    ))) {
        let response = scoped(&router, method, &uri, Some(dude), body).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "a profile change must not expose prior profile history: {uri}"
        );
    }
    assert!(state.agent_framework.close().await.is_success());
}
