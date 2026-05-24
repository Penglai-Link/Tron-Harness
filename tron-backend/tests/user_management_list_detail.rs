mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

use common::{app as auth_test_app, authenticated_request};

#[tokio::test]
async fn list_users_defaults_to_active_scope_with_paged_envelope() {
    let response = request(Method::GET, "/api/v1/admin/users?page=1&size=20", None).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-01 RED: list endpoint must exist and return HTTP 200 before active-only pagination contract can be verified",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert!(json["data"]["current"].is_number());
    assert!(json["data"]["size"].is_number());
    assert!(json["data"]["total"].is_number());

    let records = json["data"]["records"]
        .as_array()
        .expect("records must be an array");
    assert!(
        !records.is_empty(),
        "fixtures must include at least one active user",
    );

    let contains_archived = records
        .iter()
        .filter_map(|record| record.get("status").and_then(Value::as_str))
        .any(|status| status == "Archived");
    assert!(
        !contains_archived,
        "default list must exclude archived users",
    );
}

#[tokio::test]
async fn user_detail_resolves_by_user_code() {
    let response = request(Method::GET, "/api/v1/admin/users/USR-0001", None).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-01 RED: detail endpoint must resolve by stable userCode",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert_eq!(json["data"]["userCode"], "USR-0001");
    assert!(
        json["data"]["id"].is_string(),
        "internal ID must be serialized as string"
    );
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
