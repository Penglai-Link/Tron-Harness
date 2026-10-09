mod common;
use axum::http::{Method, StatusCode};
use common::{
    app_with_state_setup, authenticated_bearer_token, prepared_config, request_with_headers,
    response_json,
};
use eliteforge_agent_official_plugin_a2ui::{
    A2uiClientCapabilities, A2uiService, AGENT_CATALOG_ID, VERSION,
};
use serde_json::json;
use tron_backend::config::AppConfig;
#[tokio::test]
async fn authenticated_snapshots_conflicts_and_workspace_isolation() {
    let mut host = None;
    let app = app_with_state_setup(prepared_config(AppConfig::default()), |s| {
        host = Some(s.agent_framework.clone())
    });
    let token = authenticated_bearer_token(&app).await;
    let created = response_json(
        request_with_headers(
            &app,
            Method::POST,
            "/api/v1/chats/sessions",
            Some(json!({"workspaceCode":"TRON_CORE","agentCode":"main-agent","title":"UI"})),
            None,
            Some(&token),
        )
        .await,
    )
    .await;
    let session = created["data"]["sessionCode"]
        .as_str()
        .expect("created session");
    let path = format!("/api/v1/chats/sessions/{session}/a2ui-state");
    let empty = response_json(
        request_with_headers(&app, Method::GET, &path, None, None, Some(&token)).await,
    )
    .await;
    assert_eq!(empty["data"]["surfaces"], json!([]));
    let f = host.unwrap().storage_framework().await.unwrap();
    let service = f.services().unwrap().get::<A2uiService>().unwrap();
    let input = json!({"mode":"Display","messages":[{"version":VERSION,"createSurface":{"surfaceId":"report","catalogId":AGENT_CATALOG_ID}},{"version":VERSION,"updateComponents":{"surfaceId":"report","components":[{"id":"root","component":"Text","text":"Durable"}]}}]});
    service
        .publish(
            session,
            "main-agent",
            "run",
            "invocation",
            &A2uiClientCapabilities {
                version: VERSION.into(),
                catalog_ids: vec![AGENT_CATALOG_ID.into()],
            },
            input,
        )
        .await
        .unwrap();
    let snapshot = response_json(
        request_with_headers(&app, Method::GET, &path, None, None, Some(&token)).await,
    )
    .await;
    assert_eq!(snapshot["data"]["revision"], 1);
    assert_eq!(
        snapshot["data"]["surfaces"][0]["snapshot"]["components"]["root"]["text"],
        "Durable"
    );
    let denied = request_with_headers(
        &app,
        Method::GET,
        &format!("{path}?workspaceCode=COMPLIANCE"),
        None,
        None,
        Some(&token),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
    let action = json!({"actionId":"a","surfaceInstanceId":snapshot["data"]["surfaces"][0]["surfaceInstanceId"],"expectedRevision":1,"message":{"version":VERSION,"action":{"surfaceId":"report","sourceComponentId":"root","name":"submit","timestamp":"2026-10-09T12:00:00Z","context":{}}},"metadata":{}});
    let rejected = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session}/a2ui/actions/stream"),
        Some(action),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(rejected).await["code"], "101031002");
    let archived = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session}/archive"),
        None,
        None,
        Some(&token),
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let detail = response_json(
        request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/admin/archived-sessions/{session}"),
            None,
            None,
            Some(&token),
        )
        .await,
    )
    .await;
    assert_eq!(
        detail["data"]["a2uiState"]["surfaces"][0]["snapshot"]["components"]["root"]["text"],
        "Durable"
    );
}

// Match the server runtime: archive reads include synchronous product repositories.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_a2ui_keeps_sessions_and_archives_available_without_installing_plugin() {
    let config = prepared_config(AppConfig::default());
    // App bootstrap initializes Framework storage, so disable before construction.
    let capability_path =
        std::path::Path::new(&config.database_url).with_extension("capabilities.json");
    std::fs::write(
        capability_path,
        r#"{"packs":{"framework.a2ui":false},"extensions":{}}"#,
    )
    .unwrap();
    let mut host = None;
    let app = app_with_state_setup(config, |state| {
        host = Some(state.agent_framework.clone());
    });
    let host = host.unwrap();
    let token = authenticated_bearer_token(&app).await;
    let created = response_json(request_with_headers(
        &app, Method::POST, "/api/v1/chats/sessions",
        Some(json!({"workspaceCode":"TRON_CORE","agentCode":"main-agent","title":"Without A2UI"})),
        None, Some(&token),
    ).await).await;
    let session = created["data"]["sessionCode"].as_str().unwrap();
    let framework = host.storage_framework().await.unwrap();
    assert!(framework.services().unwrap().get::<A2uiService>().is_none());
    let snapshot = response_json(
        request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/chats/sessions/{session}/a2ui-state"),
            None,
            None,
            Some(&token),
        )
        .await,
    )
    .await;
    assert_eq!(snapshot["data"]["surfaces"], json!([]));
    assert_eq!(snapshot["data"]["revision"], 0);
    let archived = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session}/archive"),
        None,
        None,
        Some(&token),
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let detail = response_json(
        request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/admin/archived-sessions/{session}"),
            None,
            None,
            Some(&token),
        )
        .await,
    )
    .await;
    assert_eq!(detail["data"]["a2uiState"]["surfaces"], json!([]));
}
