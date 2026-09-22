#![allow(dead_code)]

use std::{path::PathBuf, sync::{atomic::{AtomicU64, Ordering}, Arc}, time::{SystemTime, UNIX_EPOCH}};

use axum::{body::{to_bytes, Body}, http::{header, HeaderMap, HeaderValue, Method, Request}, Router};
use serde_json::{json, Value};
use tower::ServiceExt;
use tron_backend::{
    adapters::outbound::persistence::{SqliteAppRepository, SqliteVariableRepository, SqliteWorkspaceRepository},
    app::build_router_with_id_provider,
    application::{app::AppRepository, variable::VariableRepository, workspace::WorkspaceRepository},
    config::{AppConfig, VariableCryptoConfig},
    domain::{
        app::{App, AppRuntimeRole, AppStatus, AppTag},
        variable::Variable,
        workspace::{Workspace, WorkspaceStatus},
    },
    shared::id::{FixedSnowflakeIdProvider, SnowflakeId},
};

static TEST_DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static TEST_ID_SEQUENCE: AtomicU64 = AtomicU64::new(734_900_000_000_000_000);
const FIXTURE_TIMESTAMP: &str = "2026-05-16T00:00:00Z";

pub const AUTH_EMAIL: &str = "profile.operator@tron.ai";
pub const AUTH_LOGIN: &str = "profile.operator";
pub const AUTH_PASSWORD: &str = "unused-profile-auth";
pub const OPEN_API_TEST_APP_CODE: &str = "OPENAPP-FIXTURE-0001";
pub const OPEN_API_TEST_APP_KEY: &str = "unused-open-api-key";
pub const OPEN_API_TEST_USER_CODE: &str = "PERSONAI-PROFILE-USER";

pub fn app() -> Router {
    build_router_with_test_ids(prepared_config(AppConfig::default()))
}

pub fn app_with_config(config: AppConfig) -> Router {
    build_router_with_test_ids(prepared_config(config))
}

fn build_router_with_test_ids(config: AppConfig) -> Router {
    build_router_with_id_provider(config, Arc::new(FixedSnowflakeIdProvider::new(next_test_id_seed())))
        .expect("test router should build")
}

pub fn prepared_config(mut config: AppConfig) -> AppConfig {
    if config.database_url == ":memory:" {
        config.database_url = temp_database_path("integration").display().to_string();
    }
    if config.variable_crypto.key_base64.trim().is_empty() {
        config.variable_crypto = VariableCryptoConfig::test_default();
    }
    seed_default_fixture_data(&config.database_url);
    config
}

fn next_test_id_seed() -> i64 {
    TEST_ID_SEQUENCE.fetch_add(10_000, Ordering::Relaxed) as i64
}

fn seed_default_fixture_data(database_url: &str) {
    seed_workspaces(database_url);
    seed_apps(database_url);
    seed_variables(database_url);
}

fn seed_workspaces(database_url: &str) {
    let mut repository = SqliteWorkspaceRepository::connect(database_url).expect("workspace repository");
    if repository.list_workspaces().expect("workspace list").is_empty() {
        for workspace in [
            ("ENTERPRISE", "Enterprise", None),
            ("PLATFORM", "Platform", Some("ENTERPRISE")),
            ("TRON_CORE", "Tron Core", Some("PLATFORM")),
            ("COMPLIANCE", "Compliance", None),
        ] {
            repository.create_workspace(&Workspace {
                id: SnowflakeId::new(next_test_id_seed()),
                workspace_code: workspace.0.into(),
                name: workspace.1.into(),
                description: Some(format!("{} workspace", workspace.1)),
                parent_workspace_code: workspace.2.map(str::to_owned),
                status: WorkspaceStatus::Active,
                created_at: FIXTURE_TIMESTAMP.into(),
                updated_at: FIXTURE_TIMESTAMP.into(),
                archived_at: None,
            }).expect("workspace fixture");
        }
    }
}

fn seed_apps(database_url: &str) {
    let mut repository = SqliteAppRepository::connect(database_url).expect("app repository");
    if repository.list_apps().expect("app list").is_empty() {
        let app = App {
            id: SnowflakeId::new(next_test_id_seed()),
            app_code: "APP-TRON-CONSOLE".into(),
            workspace_code: "TRON_CORE".into(),
            name: "Tron Console".into(),
            description: Some("Test console application".into()),
            local_directory: "/workspace/tron".into(),
            runtime_role: AppRuntimeRole::Frontend,
            repository_url: None,
            default_branch: Some("master".into()),
            status: AppStatus::Active,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
            archived_at: None,
        };
        repository.create_app(&app).expect("app fixture");
        repository.replace_app_tags(&app.app_code, &[AppTag { app_code: app.app_code.clone(), tag: "Test".into(), linked_at: FIXTURE_TIMESTAMP.into() }]).expect("app tags fixture");
    }
}

fn seed_variables(database_url: &str) {
    let mut repository = SqliteVariableRepository::connect(database_url).expect("variable repository");
    if repository.list_variables().expect("variable list").is_empty() {
        repository.create_variable(&Variable {
            id: SnowflakeId::new(next_test_id_seed()),
            workspace_code: "TRON_CORE".into(),
            key: "ROUTER_TIMEOUT_MS".into(),
            value: "400".into(),
            description: Some("Test router timeout".into()),
            is_sensitive: false,
            value_nonce: None,
            value_tag: None,
            value_algorithm: None,
            value_key_version: None,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
        }).expect("variable fixture");
    }
}

pub fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let sequence = TEST_DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("tron-tests-{test_name}-{}-{nanos}-{sequence}.sqlite3", std::process::id()))
}

pub async fn request(app: &Router, method: Method, uri: &str, body: Option<Value>, cookie_header: Option<&str>) -> axum::response::Response {
    request_with_headers(app, method, uri, body, cookie_header, None).await
}

pub async fn request_with_headers(app: &Router, method: Method, uri: &str, body: Option<Value>, cookie_header: Option<&str>, authorization_header: Option<&str>) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookies) = cookie_header { builder = builder.header(header::COOKIE, cookies); }
    if let Some(authorization) = authorization_header { builder = builder.header(header::AUTHORIZATION, authorization); }
    let request_body = if let Some(json_body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json_body.to_string())
    } else { Body::empty() };
    app.clone().oneshot(builder.body(request_body).expect("request")).await.expect("router response")
}

pub async fn authenticated_request(app: &Router, method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    request_with_headers(app, method, uri, body, None, Some("Bearer profile-test-token")).await
}

pub async fn authenticated_request_with_config(config: AppConfig, method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    authenticated_request(&app_with_config(config), method, uri, body).await
}

pub async fn authenticated_bearer_token(_app: &Router) -> String { "profile-test-token".into() }

pub async fn authenticated_refresh_cookie_header(_app: &Router) -> String { "".into() }

pub async fn signed_open_api_request(app: &Router, method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    request_with_headers(app, method, uri, body, None, Some("Bearer profile-test-token")).await
}

pub async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    serde_json::from_slice(&bytes).expect("json response")
}

pub fn header_values(headers: &HeaderMap, name: header::HeaderName) -> Vec<String> {
    headers.get_all(name).iter().map(header_value_to_string).collect()
}

pub fn header_value_to_string(value: &HeaderValue) -> String { value.to_str().unwrap_or_default().to_owned() }

pub fn cookie_header_from_set_cookie(set_cookie_headers: &[String], cookie_names: &[&str]) -> String {
    set_cookie_headers.iter().filter_map(|header| header.split(';').next()).filter(|cookie| cookie_names.iter().any(|name| cookie.starts_with(&format!("{name}=")))).collect::<Vec<_>>().join("; ")
}

pub fn cookie_value_from_set_cookie(set_cookie_headers: &[String], cookie_name: &str) -> String {
    set_cookie_headers.iter().find_map(|header| header.split(';').next().and_then(|cookie| cookie.strip_prefix(&format!("{cookie_name}="))).map(str::to_owned)).unwrap_or_default()
}
