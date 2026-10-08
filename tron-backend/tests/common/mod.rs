#![allow(dead_code)]

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Method, Request, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{
    adapters::outbound::persistence::{
        SqliteAppRepository, SqliteVariableRepository, SqliteWorkspaceRepository,
    },
    app::{AppState, build_router_with_state},
    application::{
        app::AppRepository, variable::VariableRepository, workspace::WorkspaceRepository,
    },
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
    app_with_state_setup(config, |_| {})
}

pub fn app_with_state_setup(config: AppConfig, setup: impl FnOnce(&AppState)) -> Router {
    let profiles = tron_backend::application::profile::ProfileRegistryService::new(":memory:")
        .expect("isolated profiles");
    profiles
        .register_or_update("USR-AUTH-0001", "profile.operator")
        .expect("active test profile");
    let state = AppState::new_with_profiles(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(next_test_id_seed())),
        profiles,
    )
    .expect("test state");
    setup(&state);
    build_router_with_state(state).expect("test router should build")
}

pub fn prepared_config(mut config: AppConfig) -> AppConfig {
    if config.database_url == ":memory:" {
        config.database_url = temp_database_path("integration").display().to_string();
    }
    if config.variable_crypto.key_base64.trim().is_empty() {
        config.variable_crypto = VariableCryptoConfig::test_default();
    }
    let defaults = AppConfig::default().system_agents;
    if config.system_agents.codex_command == defaults.codex_command {
        config.system_agents.codex_command.clear();
    }
    if config.system_agents.opencode_command == defaults.opencode_command {
        config.system_agents.opencode_command.clear();
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
    let mut repository =
        SqliteWorkspaceRepository::connect(database_url).expect("workspace repository");
    if repository
        .list_workspaces()
        .expect("workspace list")
        .is_empty()
    {
        for workspace in [
            ("ENTERPRISE", "Enterprise", None),
            ("PLATFORM", "Platform", Some("ENTERPRISE")),
            ("TRON_CORE", "Tron Core", Some("PLATFORM")),
            ("COMPLIANCE", "Compliance", None),
        ] {
            repository
                .create_workspace(&Workspace {
                    id: SnowflakeId::new(next_test_id_seed()),
                    workspace_code: workspace.0.into(),
                    name: workspace.1.into(),
                    description: Some(format!("{} workspace", workspace.1)),
                    parent_workspace_code: workspace.2.map(str::to_owned),
                    status: WorkspaceStatus::Active,
                    created_at: FIXTURE_TIMESTAMP.into(),
                    updated_at: FIXTURE_TIMESTAMP.into(),
                    archived_at: None,
                })
                .expect("workspace fixture");
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
        repository
            .replace_app_tags(
                &app.app_code,
                &[AppTag {
                    app_code: app.app_code.clone(),
                    tag: "Test".into(),
                    linked_at: FIXTURE_TIMESTAMP.into(),
                }],
            )
            .expect("app tags fixture");
    }
}

fn seed_variables(database_url: &str) {
    let mut repository =
        SqliteVariableRepository::connect(database_url).expect("variable repository");
    if repository
        .list_variables()
        .expect("variable list")
        .is_empty()
    {
        // Explicit test data; production startup must remain free of demo secrets.
        for (workspace, key, value) in [
            ("ENTERPRISE", "DEFAULT_REGION", "us-east-1"),
            ("PLATFORM", "ROUTER_TIMEOUT_MS", "800"),
            ("PLATFORM", "OPENAI_API_KEY", "fixture-platform-key"),
            ("TRON_CORE", "ROUTER_TIMEOUT_MS", "400"),
            ("TRON_CORE", "OPENAI_API_KEY", "fixture-workspace-key"),
        ] {
            repository
                .create_variable(&Variable {
                    id: SnowflakeId::new(next_test_id_seed()),
                    workspace_code: workspace.into(),
                    key: key.into(),
                    value: value.into(),
                    description: Some("Integration fixture".into()),
                    is_sensitive: false,
                    value_nonce: None,
                    value_tag: None,
                    value_algorithm: None,
                    value_key_version: None,
                    created_at: FIXTURE_TIMESTAMP.into(),
                    updated_at: FIXTURE_TIMESTAMP.into(),
                })
                .expect("variable fixture");
        }
    }
}

pub fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEST_DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "tron-tests-{test_name}-{}-{nanos}-{sequence}.sqlite3",
        std::process::id()
    ))
}

pub async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie_header: Option<&str>,
) -> axum::response::Response {
    request_with_headers(app, method, uri, body, cookie_header, None).await
}

pub async fn request_with_headers(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie_header: Option<&str>,
    authorization_header: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if (uri.starts_with("/api/v1/chats/")
        || uri.starts_with("/api/v1/agents/runs")
        || uri.starts_with("/api/v1/agents/agui/streams/"))
        && !uri.contains("workspaceCode=")
    {
        let workspace = body
            .as_ref()
            .and_then(|body| body["workspaceCode"].as_str())
            .unwrap_or("TRON_CORE");
        builder = builder.header("x-workspace-code", workspace);
    }
    if let Some(cookies) = cookie_header {
        builder = builder.header(header::COOKIE, cookies);
    }
    if let Some(authorization) = authorization_header {
        builder = builder.header(header::AUTHORIZATION, authorization);
    }
    let request_body = if let Some(json_body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(builder.body(request_body).expect("request"))
        .await
        .expect("router response")
}

pub async fn authenticated_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    request_with_headers(
        app,
        method,
        uri,
        body,
        None,
        Some("Bearer profile-test-token"),
    )
    .await
}

pub async fn authenticated_request_with_config(
    config: AppConfig,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    authenticated_request(&app_with_config(config), method, uri, body).await
}

pub async fn authenticated_bearer_token(_app: &Router) -> String {
    "profile-test-token".into()
}

pub async fn authenticated_refresh_cookie_header(_app: &Router) -> String {
    "".into()
}

pub async fn signed_open_api_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    request_with_headers(
        app,
        method,
        uri,
        body,
        None,
        Some("Bearer profile-test-token"),
    )
    .await
}

pub async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("json response")
}

pub fn header_values(headers: &HeaderMap, name: header::HeaderName) -> Vec<String> {
    headers
        .get_all(name)
        .iter()
        .map(header_value_to_string)
        .collect()
}

pub fn header_value_to_string(value: &HeaderValue) -> String {
    value.to_str().unwrap_or_default().to_owned()
}

pub fn cookie_header_from_set_cookie(
    set_cookie_headers: &[String],
    cookie_names: &[&str],
) -> String {
    set_cookie_headers
        .iter()
        .filter_map(|header| header.split(';').next())
        .filter(|cookie| {
            cookie_names
                .iter()
                .any(|name| cookie.starts_with(&format!("{name}=")))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn cookie_value_from_set_cookie(set_cookie_headers: &[String], cookie_name: &str) -> String {
    set_cookie_headers
        .iter()
        .find_map(|header| {
            header
                .split(';')
                .next()
                .and_then(|cookie| cookie.strip_prefix(&format!("{cookie_name}=")))
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

/// Explicit exposure for contracts that exercise the complete schema catalog.
pub async fn enable_direct_list(app: &Router) {
    let response = authenticated_request(
        app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({"toolExposureMode":"direct_list"})),
    )
    .await;
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

pub fn app_without_profile() -> Router {
    let config = prepared_config(AppConfig::default());
    let profiles = tron_backend::application::profile::ProfileRegistryService::new(":memory:")
        .expect("isolated empty profiles");
    build_router_with_state(
        AppState::new_with_profiles(
            config,
            Arc::new(FixedSnowflakeIdProvider::new(next_test_id_seed())),
            profiles,
        )
        .expect("test state"),
    )
    .expect("test router")
}

/// Persist a product presentation through the Framework event abstraction.
pub async fn append_native_chat_message(database: &str, session: &str, message: Value) {
    let host = tron_backend::application::agent::FrameworkHost::persistent(database, None);
    let framework = host.storage_framework().await.unwrap();
    framework
        .sessions()
        .unwrap()
        .store
        .append_event(eliteforge_agent_core::SessionEvent::new(
            session,
            "tron.message",
            message,
        ))
        .await
        .unwrap();
}

pub async fn native_session_events(
    database: &str,
    session: &str,
) -> Vec<eliteforge_agent_core::SessionEvent> {
    let host = tron_backend::application::agent::FrameworkHost::persistent(database, None);
    let framework = host.storage_framework().await.unwrap();
    let store = framework.sessions().unwrap().store;
    let mut all = Vec::new();
    let mut after = 0;
    loop {
        let page = store.list_events(session, after, 500).await.unwrap();
        if page.is_empty() {
            return all;
        }
        after = page.last().unwrap().sequence;
        all.extend(page);
    }
}

pub async fn native_runs(database: &str, session: &str) -> Vec<eliteforge_agent_core::ManagedRun> {
    let host = tron_backend::application::agent::FrameworkHost::persistent(database, None);
    host.storage_framework()
        .await
        .unwrap()
        .runs()
        .unwrap()
        .list(session)
        .await
        .unwrap()
}
pub async fn native_context(database: &str, session: &str, key: &str) -> Option<Value> {
    let host = tron_backend::application::agent::FrameworkHost::persistent(database, None);
    host.storage_framework()
        .await
        .unwrap()
        .sessions()
        .unwrap()
        .store
        .read_context(session, key)
        .await
        .unwrap()
        .map(|(_, value)| value)
}
pub async fn native_goal(database: &str, id: &str) -> eliteforge_agent_core::GoalRecord {
    let host = tron_backend::application::agent::FrameworkHost::persistent(database, None);
    host.storage_framework()
        .await
        .unwrap()
        .goals()
        .unwrap()
        .get(id)
        .await
        .unwrap()
}
pub async fn wait_native_summary(database: &str, session: &str) -> Value {
    for _ in 0..250 {
        if let Some(summary) = native_context(database, session, "tron.session-summary-view").await
        {
            return summary;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("native session summary did not finish");
}
