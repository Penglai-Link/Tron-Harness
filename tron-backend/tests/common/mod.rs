use std::sync::Arc;

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderMap, HeaderValue, Method, Request, header},
    routing::{get, post},
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceExt;
use tron_backend::{
    app::build_router,
    config::{AppConfig, AuthConfig, GitHubOAuthConfig},
};

pub const AUTH_EMAIL: &str = "auth.operator@tron.ai";
pub const AUTH_LOGIN: &str = "auth.operator";
pub const AUTH_PASSWORD: &str = "TronAuthPass123!";
pub const ARCHIVED_LOGIN: &str = "archived.auth";
const GITHUB_TEST_CLIENT_ID: &str = "github-client-id";
const GITHUB_TEST_CLIENT_SECRET: &str = "github-client-secret";
const GITHUB_TEST_ACCESS_TOKEN: &str = "github-access-token";
pub const GITHUB_TEST_CODE: &str = "github-test-code";

#[derive(Clone)]
struct GitHubMockState {
    access_token: &'static str,
    profile: GitHubProfileFixture,
}

#[derive(Clone)]
pub struct GitHubEmailFixture {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

#[derive(Clone)]
pub struct GitHubProfileFixture {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub emails: Vec<GitHubEmailFixture>,
}

impl Default for GitHubProfileFixture {
    fn default() -> Self {
        Self {
            id: 734_103_002_743_185_500,
            login: AUTH_LOGIN.into(),
            name: Some("Auth Operator".into()),
            emails: vec![GitHubEmailFixture {
                email: AUTH_EMAIL.into(),
                primary: true,
                verified: true,
            }],
        }
    }
}

pub struct GitHubMockServer {
    pub authorize_url: String,
    pub token_url: String,
    pub user_api_url: String,
    pub emails_api_url: String,
    task: JoinHandle<()>,
}

pub fn app() -> Router {
    build_router(AppConfig::default())
}

pub fn app_with_config(config: AppConfig) -> Router {
    build_router(config)
}

pub async fn github_mock_server() -> GitHubMockServer {
    github_mock_server_with_profile(GitHubProfileFixture::default()).await
}

pub async fn github_mock_server_with_profile(profile: GitHubProfileFixture) -> GitHubMockServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("github mock server should bind");
    let address = listener
        .local_addr()
        .expect("github mock server should expose a local address");
    let base_url = format!("http://{address}");
    let state = Arc::new(GitHubMockState {
        access_token: GITHUB_TEST_ACCESS_TOKEN,
        profile,
    });
    let app = Router::new()
        .route("/login/oauth/access_token", post(github_token))
        .route("/api/user", get(github_user))
        .route("/api/user/emails", get(github_user_emails))
        .with_state(state);
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("github mock server should stay available during the test");
    });

    GitHubMockServer {
        authorize_url: format!("{base_url}/login/oauth/authorize"),
        token_url: format!("{base_url}/login/oauth/access_token"),
        user_api_url: format!("{base_url}/api/user"),
        emails_api_url: format!("{base_url}/api/user/emails"),
        task,
    }
}

pub fn github_oauth_config(server: &GitHubMockServer) -> AppConfig {
    let mut config = AppConfig::default();
    config.auth = AuthConfig {
        dev_oauth2_enabled: false,
        dev_oauth2_name: "OAuth2".into(),
        github_oauth: Some(GitHubOAuthConfig {
            client_id: GITHUB_TEST_CLIENT_ID.into(),
            client_secret: GITHUB_TEST_CLIENT_SECRET.into(),
            redirect_uri: "http://127.0.0.1:5173/api/v1/auth/oauth2/callback/github".into(),
            name: "GitHub".into(),
            scope: "read:user user:email".into(),
            authorize_url: server.authorize_url.clone(),
            token_url: server.token_url.clone(),
            user_api_url: server.user_api_url.clone(),
            emails_api_url: server.emails_api_url.clone(),
        }),
        ..config.auth
    };
    config
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

impl Drop for GitHubMockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn github_token(
    State(state): State<Arc<GitHubMockState>>,
) -> Json<Value> {
    Json(json!({
        "access_token": state.access_token,
        "token_type": "bearer",
        "scope": "read:user,user:email",
    }))
}

async fn github_user(State(state): State<Arc<GitHubMockState>>) -> Json<Value> {
    Json(json!({
        "id": state.profile.id,
        "login": state.profile.login,
        "name": state.profile.name,
    }))
}

async fn github_user_emails(State(state): State<Arc<GitHubMockState>>) -> Json<Value> {
    Json(json!(
        state
            .profile
            .emails
            .iter()
            .map(|email| json!({
                "email": email.email,
                "primary": email.primary,
                "verified": email.verified,
            }))
            .collect::<Vec<_>>()
    ))
}
