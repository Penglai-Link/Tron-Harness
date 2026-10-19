mod common;
use axum::http::Method;
use common::{authenticated_request, response_json};
use eliteforge_agent_core::SessionEvent;
use serde_json::{Value, json};
use std::sync::Arc;
use tron_backend::{
    app::{AppState, build_router_with_state},
    config::AppConfig,
};

#[tokio::test]
async fn saved_context_assembly_is_ordered_pinned_compact_and_session_scoped() {
    let root = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default();
    config.database_url = root.path().join("business.sqlite").display().to_string();
    config.graph_storage_path = root.path().join("graph").display().to_string();
    config.resource = tron_backend::config::ResourceConfig::with_default_root(
        root.path().join("resources").display().to_string(),
    );
    let state = AppState::new_with_id_provider(
        common::prepared_config(config),
        Arc::new(tron_backend::shared::id::FixedSnowflakeIdProvider::new(
            801700000000000000,
        )),
    )
    .unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    let app = build_router_with_state(state.clone()).unwrap();
    let response = authenticated_request(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({"workspaceCode":"TRON_CORE","agentCode":"main-agent"})),
    )
    .await;
    assert_eq!(response.status(), 201);
    let created = response_json(response).await;
    let session = created["data"]["sessionCode"].as_str().unwrap();
    let framework = state.agent_framework.storage_framework().await.unwrap();
    let payload = |used: usize, configured: Value| json!({"messages":[{"role":"system","content":"Private text must remain outside the assembly endpoint"}],"tools":[{"secret":"must not return"}],"budgetBreakdown":{"estimatedInputTokens":used},"contextReport":{"model":{"connection":"fixture","model":"fixture-model"},"stage":"chat","step":1,"attempt":1,"input_capacity":1000,"protocol_tokens":10,"estimated_input_tokens":used,"blocks":[{"id":"framework.instructions","position":0,"source_version":null,"original_tokens":used,"allocated_tokens":100,"configured_tokens":configured,"final_tokens":used,"final_message_count":1,"outcome":"Kept"}]}});
    let first = framework
        .sessions()
        .unwrap()
        .store
        .append_event(SessionEvent::new(
            session,
            "tron.context-view",
            payload(12, json!(200)),
        ))
        .await
        .unwrap();
    let second = framework
        .sessions()
        .unwrap()
        .store
        .append_event(SessionEvent::new(
            session,
            "tron.context-view",
            payload(23, Value::Null),
        ))
        .await
        .unwrap();
    framework
        .sessions()
        .unwrap()
        .store
        .append_event(SessionEvent::new(
            session,
            "unrelated",
            json!({"ignored":true}),
        ))
        .await
        .unwrap();
    let url = format!(
        "/api/v1/chats/sessions/get-context-assembly?sessionCode={session}&workspaceCode=TRON_CORE"
    );
    let read = |url: String| {
        let app = app.clone();
        async move {
            let response = authenticated_request(&app, Method::GET, &url, None).await;
            let status = response.status();
            (status, response_json(response).await)
        }
    };
    let (status, latest) = read(url.clone()).await;
    assert_eq!(status, 200, "{latest}");
    let snapshot = &latest["data"]["snapshot"];
    assert_eq!(snapshot["snapshotSequence"], second.sequence.to_string());
    assert_eq!(snapshot["blocks"][0]["finalTokens"], 23);
    assert!(snapshot["blocks"][0]["configuredTokens"].is_null());
    assert!(!latest.to_string().contains("Private text"));
    assert!(!latest.to_string().contains("must not return"));
    let (_, pinned) = read(format!("{url}&snapshotSequence={}", first.sequence)).await;
    assert_eq!(
        pinned["data"]["snapshot"]["blocks"][0]["configuredTokens"],
        200
    );
    assert_eq!(
        pinned["data"]["snapshot"]["blocks"][0]["allocatedTokens"],
        100
    );
    assert_eq!(pinned["data"]["snapshot"]["blocks"][0]["finalTokens"], 12);
    assert_eq!(read(format!("{url}&snapshotSequence=0")).await.0, 400);
    assert_eq!(read(format!("/api/v1/chats/sessions/get-context-assembly?sessionCode={session}&workspaceCode=UNKNOWN")).await.0,404);
    let detail=read(format!("/api/v1/chats/sessions/get-context-detail?sessionCode={session}&workspaceCode=TRON_CORE&snapshotSequence={}&blockCode=systemPrompt",first.sequence)).await;
    assert_eq!(detail.0, 200);
    assert!(detail.1.to_string().contains("Private text"));
    assert_eq!(
        read(url.clone()).await.1,
        latest,
        "reading details must not change the selected report"
    );
}
