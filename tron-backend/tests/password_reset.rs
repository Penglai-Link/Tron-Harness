mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

use common::{app as auth_test_app, authenticated_request};

#[tokio::test]
async fn reset_password_returns_temporary_password_once_without_persisting_plaintext() {
    let router = auth_test_app();
    let reset_response = request(
        &router,
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

    let detail_response = request(&router, Method::GET, "/api/v1/admin/users/USR-1001", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_json = response_json(detail_response).await;
    assert!(
        detail_json["data"].get("temporaryPassword").is_none(),
        "temporary password must not be re-readable from user detail",
    );
    assert!(
        detail_json["data"].get("passwordHash").is_none(),
        "password hash must never be exposed by user detail DTO",
    );
    assert!(
        detail_json["data"]["password"]["resetRequired"]
            .as_bool()
            .unwrap_or(false)
            || detail_json["data"]["passwordResetRequired"]
                .as_bool()
                .unwrap_or(false),
        "safe password reset-required state should remain visible",
    );
}

#[tokio::test]
async fn user_detail_never_exposes_password_hash() {
    let router = auth_test_app();
    let detail_response = request(&router, Method::GET, "/api/v1/admin/users/USR-1001", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_json = response_json(detail_response).await;
    assert!(
        detail_json["data"].get("passwordHash").is_none(),
        "password hash must never be returned by API DTOs",
    );
}

async fn request(
    router: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    authenticated_request(router, method, uri, body).await
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}
