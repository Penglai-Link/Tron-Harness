use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn reset_password_returns_temporary_password_once_without_persisting_plaintext() {
    let reset_response = request(
        Method::POST,
        "/api/v1/admin/users/USR-1001/password/reset",
        Some(json!({})),
    )
    .await;

    assert_eq!(
        reset_response.status(),
        StatusCode::OK,
        "TDD-UM-05 RED: reset-password endpoint must exist and return one-time temporary password",
    );

    let reset_json = response_json(reset_response).await;
    assert_eq!(reset_json["code"], "SUCCESS");
    let temporary_password = reset_json["data"]["temporaryPassword"]
        .as_str()
        .expect("temporary password should be emitted exactly once");
    assert!(
        !temporary_password.trim().is_empty(),
        "temporary password cannot be blank",
    );

    let detail_response = request(Method::GET, "/api/v1/admin/users/USR-1001", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_json = response_json(detail_response).await;
    assert!(
        detail_json["data"]["passwordHash"].as_str().is_some(),
        "server-side persisted password must be hash-only",
    );
    assert!(
        detail_json["data"].get("temporaryPassword").is_none(),
        "temporary password must not be re-readable from user detail",
    );
}

async fn request(method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    let router = build_router(AppConfig::default());
    let mut builder = Request::builder().method(method).uri(uri);
    let request_body = if let Some(json_body) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    router
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}
