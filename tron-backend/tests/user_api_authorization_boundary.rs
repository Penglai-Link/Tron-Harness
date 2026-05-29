mod common;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use common::{app as auth_test_app, authenticated_request};

#[tokio::test]
async fn list_user_endpoint_does_not_require_permission_header_in_mvp() {
    let response = request(Method::GET, "/api/v1/admin/users?page=1&size=20", None).await;
    let status = response.status();

    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "TDD-UM-07 RED: user list endpoint must not be blocked by role or permission data gate",
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "TDD-UM-07 RED: authenticated list endpoint should not require extra role or permission headers in MVP",
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
        StatusCode::FORBIDDEN,
        "TDD-UM-07 RED: role assignment endpoint should manage data, not enforce permission gate from role assignments",
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "TDD-UM-07 RED: authenticated endpoint is expected to accept managed role replacement in MVP scope",
    );
}

async fn request(method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    let router = auth_test_app();
    authenticated_request(&router, method, uri, body).await
}
