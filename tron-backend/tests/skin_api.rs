use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{
    adapters::inbound::http::{auth::RequestAuthContext, skins},
    app::AppState,
    application::profile::CurrentProfileContext,
    config::AppConfig,
};

fn router(root: &std::path::Path) -> (Router, AppState) {
    let state = AppState::new(AppConfig {
        application_config_path: root.join("application.yaml").display().to_string(),
        ..AppConfig::default()
    })
    .unwrap();
    let auth = RequestAuthContext::from_profile(
        &CurrentProfileContext {
            profile_id: "skin-test".into(),
            person_ai_user_id: "skin-test".into(),
            person_ai_username: "skin-test".into(),
        },
        "en-US".into(),
    );
    (
        skins::routes()
            .layer(Extension(auth))
            .with_state(state.clone()),
        state,
    )
}

async fn request(router: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn skin_api_import_activate_restart_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let (app, state) = router(dir.path());
    let (_, active) = request(&app, "GET", "/api/v1/system/skins/active", Value::Null).await;
    assert_eq!(active["code"], "0");
    assert_eq!(active["data"]["skin"]["id"], "light");
    let mut changes = state.skins.subscribe();
    let manifest = json!({"id":"ocean", "name":"Ocean", "version":1,"mode":"Light",
        "colors":{"--color-primary":"#164b85"}, "layout":{}, "resources":{"background":null,"font":null}});
    let (status, installed) = request(&app, "POST", "/api/v1/system/skins", manifest.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(installed["data"]["skin"]["id"], "ocean");
    changes.try_recv().unwrap();
    let (status, _) = request(
        &app,
        "PUT",
        "/api/v1/system/skins/active",
        json!({"id":"ocean"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    changes.try_recv().unwrap();
    let (reopened, _) = router(dir.path());
    let (_, active) = request(&reopened, "GET", "/api/v1/system/skins/active", Value::Null).await;
    assert_eq!(active["data"]["skin"]["id"], "ocean");
    let (status, error) = request(&app, "POST", "/api/v1/system/skins", manifest).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["code"], "101025003");
    assert_eq!(error["data"], json!({}));
    let (status, _) = request(
        &app,
        "PUT",
        "/api/v1/system/skins/active",
        json!({"id":"missing"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(state.skins.active().unwrap().id, "ocean");
    assert!(changes.try_recv().is_err());
    let (_, list) = request(&app, "GET", "/api/v1/system/skins", Value::Null).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn skin_api_resource_mime_and_sse() {
    let dir = tempfile::tempdir().unwrap();
    let (app, state) = router(dir.path());
    state.skins.active().unwrap();
    std::fs::write(dir.path().join("skins/light/bg.png"), b"png").unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/skins/light/resources/bg.png")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/skins/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
}

#[tokio::test]
async fn skin_api_rejects_invalid_and_oversized_bodies_with_envelopes() {
    let dir = tempfile::tempdir().unwrap();
    let (app, _) = router(dir.path());
    for (method, path, body, expected_status, code) in [
        (
            "PUT",
            "/api/v1/system/skins/active",
            "{".to_owned(),
            StatusCode::BAD_REQUEST,
            "101025001",
        ),
        (
            "POST",
            "/api/v1/system/skins",
            "x".repeat(20 * 1024 * 1024 + 1),
            StatusCode::PAYLOAD_TOO_LARGE,
            "101025005",
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected_status);
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let error: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(error["code"], code);
        assert_eq!(error["data"], json!({}));
    }
}
