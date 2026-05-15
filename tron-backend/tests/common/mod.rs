use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Method, Request, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

pub const AUTH_EMAIL: &str = "auth.operator@tron.ai";
pub const AUTH_LOGIN: &str = "auth.operator";
pub const AUTH_PASSWORD: &str = "TronAuthPass123!";
pub const ARCHIVED_LOGIN: &str = "archived.auth";

pub fn app() -> Router {
    build_router(AppConfig::default())
}

pub fn app_with_config(config: AppConfig) -> Router {
    build_router(config)
}

pub async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie_header: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookies) = cookie_header {
        builder = builder.header(header::COOKIE, cookies);
    }
    let request_body = if let Some(json_body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    app.clone()
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

pub async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}

pub async fn authenticated_cookie_header(app: &Router) -> String {
    let login_response = request(
        app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_LOGIN,
            "password": AUTH_PASSWORD,
        })),
        None,
    )
    .await;
    assert_eq!(
        login_response.status(),
        axum::http::StatusCode::OK,
        "test auth bootstrap should obtain a valid login session",
    );
    cookie_header_from_set_cookie(
        &header_values(login_response.headers(), header::SET_COOKIE),
        &["tron_access_token", "tron_refresh_token"],
    )
}

pub async fn authenticated_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let cookie_header = authenticated_cookie_header(app).await;
    request(app, method, uri, body, Some(&cookie_header)).await
}

pub async fn authenticated_request_with_config(
    config: AppConfig,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let router = app_with_config(config);
    authenticated_request(&router, method, uri, body).await
}

pub fn header_values(headers: &HeaderMap, name: header::HeaderName) -> Vec<String> {
    headers
        .get_all(name)
        .iter()
        .map(header_value_to_string)
        .collect()
}

pub fn header_value_to_string(value: &HeaderValue) -> String {
    value
        .to_str()
        .expect("header value should be valid text")
        .to_owned()
}

pub fn cookie_header_from_set_cookie(set_cookie_headers: &[String], cookie_names: &[&str]) -> String {
    cookie_names
        .iter()
        .map(|name| {
            set_cookie_headers
                .iter()
                .find_map(|cookie| {
                    let head = cookie.split(';').next()?.trim();
                    head.strip_prefix(&format!("{name}="))
                        .map(|value| format!("{name}={value}"))
                })
                .unwrap_or_else(|| panic!("expected Set-Cookie header for {name}"))
        })
        .collect::<Vec<_>>()
        .join("; ")
}
