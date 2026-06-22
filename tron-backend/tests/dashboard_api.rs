mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use common::{app, authenticated_request, response_json};

#[tokio::test]
async fn dashboard_overview_returns_frontend_payload_shape() {
    let router = app();

    let response =
        authenticated_request(&router, Method::GET, "/api/v1/dashboard/overview", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "SUCCESS");
    assert_eq!(payload["message"], "ok");
    assert!(payload["data"]["metrics"].is_array());
    assert!(payload["data"]["activities"].is_array());
    assert_eq!(
        payload["data"]["activities"][0]["tone"],
        json!("success"),
        "dashboard activity tone must match frontend union values",
    );
}
