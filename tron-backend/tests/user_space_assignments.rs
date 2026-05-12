use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn replace_spaces_rejects_blank_or_malformed_codes() {
    let response = request(
        Method::PUT,
        "/api/v1/admin/users/USR-0001/spaces",
        Some(json!({
            "spaceCodes": ["", "space without slash"]
        })),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "TDD-UM-04 RED: invalid space codes should be rejected",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "ADMIN_USER_SPACE_CODE_INVALID");
    assert!(json["message"].is_string());
}

#[tokio::test]
async fn failed_space_replacement_must_not_overwrite_previous_assignments() {
    let set_valid_response = request(
        Method::PUT,
        "/api/v1/admin/users/USR-0002/spaces",
        Some(json!({
            "spaceCodes": ["SPACE-ONE", "SPACE-TWO"]
        })),
    )
    .await;
    assert_eq!(
        set_valid_response.status(),
        StatusCode::OK,
        "TDD-UM-04 RED: valid replacement endpoint is required before rejection preservation can be verified",
    );

    let set_invalid_response = request(
        Method::PUT,
        "/api/v1/admin/users/USR-0002/spaces",
        Some(json!({
            "spaceCodes": ["  "]
        })),
    )
    .await;
    assert_eq!(set_invalid_response.status(), StatusCode::BAD_REQUEST);

    let detail_response = request(Method::GET, "/api/v1/admin/users/USR-0002", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_json = response_json(detail_response).await;
    assert_eq!(
        detail_json["data"]["spaceCodes"],
        json!(["SPACE-ONE", "SPACE-TWO"]),
        "failed replacement should not mutate previous assignments",
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
