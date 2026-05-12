use std::collections::BTreeSet;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn default_router_builds_and_serves_health() {
    let (status, json) = request_health().await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["data"]["status"], "ok");
    assert_eq!(json["data"]["service"], "tron-backend");
}

#[tokio::test]
async fn health_response_uses_uniform_success_envelope() {
    let (status, json) = request_health().await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["code"], "SUCCESS");
    assert_eq!(json["message"], "ok");
    assert!(json.get("data").is_some(), "response must include data");
    assert_eq!(
        json.as_object()
            .expect("health response must be a JSON object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["code", "data", "message"]),
    );
}

#[tokio::test]
async fn health_payload_contains_only_safe_static_metadata() {
    let (_status, json) = request_health().await;
    let data = json["data"]
        .as_object()
        .expect("health response data must be an object");

    assert_eq!(
        data.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["service", "status", "version"]),
    );

    let version = data["version"]
        .as_str()
        .expect("health response version must be a string");
    assert!(!version.trim().is_empty(), "version must not be empty");

    assert_no_sensitive_keys(&json["data"]);
}

async fn request_health() -> (StatusCode, Value) {
    let router = build_router(AppConfig::default());
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("health request should build"),
        )
        .await
        .expect("health request should be handled by the router");

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("health response body should be readable");
    let json = serde_json::from_slice(&body).expect("health response should be JSON");

    (status, json)
}

fn assert_no_sensitive_keys(value: &Value) {
    const FORBIDDEN_KEY_PARTS: &[&str] = &[
        "auth",
        "cookie",
        "credential",
        "database",
        "env",
        "host",
        "key",
        "password",
        "port",
        "secret",
        "session",
        "tenant",
        "token",
        "url",
        "user",
    ];

    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let normalized = key.to_ascii_lowercase();
                assert!(
                    !FORBIDDEN_KEY_PARTS
                        .iter()
                        .any(|forbidden| normalized.contains(forbidden)),
                    "health payload must not expose sensitive key `{key}`",
                );
                assert_no_sensitive_keys(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_sensitive_keys(item);
            }
        }
        _ => {}
    }
}
