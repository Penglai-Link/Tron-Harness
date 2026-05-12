use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn create_user_returns_stringified_id_and_user_code() {
    let response = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(json!({
            "username": "alex.johnson",
            "displayName": "Alex Johnson",
            "email": "alex.johnson@tron.ai"
        })),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "TDD-UM-02 RED: create endpoint must exist and return 201 with lifecycle payload",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert!(json["data"]["id"].is_string(), "Snowflake ID must be a decimal string");
    assert!(json["data"]["userCode"].is_string(), "stable userCode must be returned");
}

#[tokio::test]
async fn archive_then_restore_transitions_user_status() {
    let archive_response = request(
        Method::POST,
        "/api/v1/admin/users/USR-0009/archive",
        None,
    )
    .await;
    assert_eq!(
        archive_response.status(),
        StatusCode::OK,
        "TDD-UM-02 RED: archive transition endpoint is missing",
    );

    let restore_response = request(
        Method::POST,
        "/api/v1/admin/users/USR-0009/restore",
        None,
    )
    .await;
    assert_eq!(
        restore_response.status(),
        StatusCode::OK,
        "TDD-UM-02 RED: restore transition endpoint is missing",
    );

    let detail_response = request(Method::GET, "/api/v1/admin/users/USR-0009", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let json = response_json(detail_response).await;
    assert_eq!(json["data"]["status"], "Active");
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
