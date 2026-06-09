mod common;

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::to_bytes,
    http::{Method, StatusCode, header},
};
use serde_json::Value;
use tron_backend::config::AppConfig;

use common::{app_with_config, request, response_json};

static FRONTEND_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn backend_serves_spa_entry_for_root_and_client_routes() {
    let frontend_dir = write_frontend_fixture("spa-entry");
    let app = app_with_frontend_dir(&frontend_dir);

    for path in ["/", "/dashboard", "/settings/models"] {
        let response = request(&app, Method::GET, path, None, None).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_content_type(response.headers(), "text/html");
        assert!(
            response_text(response)
                .await
                .contains("tron integrated fixture"),
            "{path} should return the SPA entry point",
        );
    }

    let _ = fs::remove_dir_all(frontend_dir);
}

#[tokio::test]
async fn backend_serves_spa_entry_for_head_routes_without_body() {
    let frontend_dir = write_frontend_fixture("spa-head");
    let app = app_with_frontend_dir(&frontend_dir);

    let response = request(&app, Method::HEAD, "/dashboard", None, None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_content_type(response.headers(), "text/html");
    assert_eq!(response_text(response).await, "");

    let _ = fs::remove_dir_all(frontend_dir);
}

#[tokio::test]
async fn backend_serves_frontend_assets_from_resource_directory() {
    let frontend_dir = write_frontend_fixture("asset");
    let app = app_with_frontend_dir(&frontend_dir);

    let response = request(&app, Method::GET, "/assets/index-test.js", None, None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_content_type(response.headers(), "text/javascript");
    assert_eq!(
        response_text(response).await,
        "window.__TRON_TEST_ASSET__ = true;\n"
    );

    let _ = fs::remove_dir_all(frontend_dir);
}

#[tokio::test]
async fn non_get_frontend_routes_return_405_instead_of_spa_html() {
    let frontend_dir = write_frontend_fixture("spa-method-guard");
    let app = app_with_frontend_dir(&frontend_dir);

    let response = request(&app, Method::POST, "/dashboard", None, None).await;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_content_type(response.headers(), "text/plain");
    assert_eq!(
        response
            .headers()
            .get(header::ALLOW)
            .and_then(|value| value.to_str().ok()),
        Some("GET, HEAD"),
    );
    assert!(
        !response_text(response)
            .await
            .contains("tron integrated fixture"),
        "non-GET frontend requests must not return the SPA entry point",
    );

    let _ = fs::remove_dir_all(frontend_dir);
}

#[tokio::test]
async fn unknown_api_routes_return_json_not_spa_html() {
    let frontend_dir = write_frontend_fixture("api-boundary");
    let app = app_with_frontend_dir(&frontend_dir);

    let response = request(&app, Method::GET, "/api/v1/not-exists", None, None).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_content_type(response.headers(), "application/json");
    let payload = response_json(response).await;
    assert_eq!(payload["code"], Value::String("NOT_FOUND".into()));
    assert_eq!(payload["data"], serde_json::json!({}));

    let _ = fs::remove_dir_all(frontend_dir);
}

#[tokio::test]
async fn unknown_api_post_routes_still_return_json_not_found() {
    let frontend_dir = write_frontend_fixture("api-boundary-post");
    let app = app_with_frontend_dir(&frontend_dir);

    let response = request(&app, Method::POST, "/api/v1/not-exists", None, None).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_content_type(response.headers(), "application/json");
    let payload = response_json(response).await;
    assert_eq!(payload["code"], Value::String("NOT_FOUND".into()));
    assert_eq!(payload["data"], serde_json::json!({}));

    let _ = fs::remove_dir_all(frontend_dir);
}

#[tokio::test]
async fn known_api_method_mismatch_still_returns_405() {
    let frontend_dir = write_frontend_fixture("api-method-guard");
    let app = app_with_frontend_dir(&frontend_dir);

    let response = request(&app, Method::POST, "/api/v1/health", None, None).await;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let _ = fs::remove_dir_all(frontend_dir);
}

fn app_with_frontend_dir(frontend_dir: &std::path::Path) -> axum::Router {
    let mut config = AppConfig::default();
    config.frontend_dist_dir = frontend_dir.display().to_string();
    app_with_config(config)
}

fn write_frontend_fixture(test_name: &str) -> PathBuf {
    let root = temp_frontend_dir(test_name);
    fs::create_dir_all(root.join("assets")).expect("frontend fixture assets dir should be created");
    fs::write(
        root.join("index.html"),
        "<!doctype html><html><body>tron integrated fixture</body></html>\n",
    )
    .expect("frontend fixture index should be written");
    fs::write(
        root.join("assets").join("index-test.js"),
        "window.__TRON_TEST_ASSET__ = true;\n",
    )
    .expect("frontend fixture asset should be written");
    root
}

fn temp_frontend_dir(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let sequence = FRONTEND_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("tron-frontend-{test_name}-{nanos}-{sequence}"))
}

fn assert_content_type(headers: &axum::http::HeaderMap, expected_prefix: &str) {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(
        content_type.starts_with(expected_prefix),
        "expected content-type `{content_type}` to start with `{expected_prefix}`",
    );
}

async fn response_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    String::from_utf8(bytes.to_vec()).expect("response body should be utf-8")
}
