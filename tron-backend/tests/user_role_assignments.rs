mod common;

use axum::{
    body::to_bytes,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};

use common::{app as auth_test_app, authenticated_request};

#[tokio::test]
async fn replace_roles_accepts_only_fixed_role_codes() {
    let response = request(
        Method::PUT,
        "/api/v1/admin/users/USR-0001/roles",
        Some(json!({
            "roleCodes": ["ADMIN", "DEVELOPER"]
        })),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-03 RED: role replacement endpoint for fixed role codes must be implemented",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert_eq!(json["data"]["userCode"], "USR-0001");
    assert_eq!(json["data"]["roleCodes"], json!(["ADMIN", "DEVELOPER"]));
}

#[tokio::test]
async fn replace_roles_rejects_unknown_role_code_with_stable_error_contract() {
    let response = request(
        Method::PUT,
        "/api/v1/admin/users/USR-0001/roles",
        Some(json!({
            "roleCodes": ["OWNER"]
        })),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "TDD-UM-03 RED: invalid role code should be rejected by API contract",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "ADMIN_USER_ROLE_CODE_INVALID");
    assert!(json["message"].is_string());
}

async fn request(method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    let router = auth_test_app();
    authenticated_request(&router, method, uri, body).await
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}
