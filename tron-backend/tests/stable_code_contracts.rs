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
async fn list_contract_uses_stable_codes_and_decimal_string_ids() {
    let response = request(Method::GET, "/api/v1/admin/users?page=1&size=20", None).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-06 RED: list endpoint must exist before stable code contract can be verified",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    let records = json["data"]["records"]
        .as_array()
        .expect("records must be an array");
    assert!(
        !records.is_empty(),
        "fixtures must include records to verify stable contracts",
    );

    let first = &records[0];
    assert!(first["userCode"].is_string());
    assert!(first["roleCodes"].is_array());
    assert!(first["spaceCodes"].is_array());

    let internal_id = first["id"]
        .as_str()
        .expect("internal ID must be a decimal string");
    assert!(
        internal_id.chars().all(|ch| ch.is_ascii_digit()),
        "internal ID should contain digits only",
    );
}

#[tokio::test]
async fn ids_beyond_js_safe_integer_are_preserved_as_exact_strings() {
    let response = request(Method::GET, "/api/v1/admin/users/USR-LARGE-ID", None).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-06 RED: detail endpoint with large Snowflake IDs must be available",
    );

    let json = response_json(response).await;
    let internal_id = json["data"]["id"]
        .as_str()
        .expect("large Snowflake ID must serialize as string");
    assert_eq!(
        internal_id, "9007199254740993",
        "ID above JavaScript safe integer must stay exact",
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
