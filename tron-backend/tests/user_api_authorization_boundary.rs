use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn list_user_endpoint_does_not_require_permission_header_in_mvp() {
    let response = request(Method::GET, "/api/v1/admin/users?page=1&size=20", None).await;
    let status = response.status();

    assert_ne!(
        status,
        StatusCode::UNAUTHORIZED,
        "TDD-UM-07 RED: user list endpoint must stay decoupled from authorization gating",
    );
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "TDD-UM-07 RED: user list endpoint must not be blocked by role/permission gate",
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "TDD-UM-07 RED: list endpoint should be callable without role/permission context in MVP",
    );
}

#[tokio::test]
async fn role_assignment_endpoint_treats_roles_as_managed_data_not_auth_gate() {
    let response = request(
        Method::PUT,
        "/api/v1/admin/users/USR-0007/roles",
        Some(json!({
            "roleCodes": ["VIEWER"]
        })),
    )
    .await;
    let status = response.status();

    assert_ne!(
        status,
        StatusCode::UNAUTHORIZED,
        "TDD-UM-07 RED: role assignment endpoint must not depend on authorization middleware in MVP",
    );
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "TDD-UM-07 RED: role assignment endpoint should manage data, not enforce permission gate",
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "TDD-UM-07 RED: endpoint is expected to accept managed role replacement in MVP scope",
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
