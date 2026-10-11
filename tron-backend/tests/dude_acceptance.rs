//! Product-boundary acceptance using real routes and an isolated persistent DB.
//! Runtime/concurrency tests belong beside these once host execution is assembled.
mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tron_backend::config::{AppConfig, ResourceConfig};

use common::{app_with_config, authenticated_request, response_json};

struct Fixture {
    _directory: tempfile::TempDir,
    config: AppConfig,
    router: Router,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("acceptance directory");
        let config = AppConfig {
            database_url: directory.path().join("dude.sqlite3").display().to_string(),
            graph_storage_path: directory.path().join("dude.grafeo").display().to_string(),
            skills_registry_root: directory.path().join("skills").display().to_string(),
            resource: ResourceConfig::with_default_root(
                directory.path().join("resources").display().to_string(),
            ),
            ..AppConfig::default()
        };
        let router = app_with_config(config.clone());
        Self {
            _directory: directory,
            config,
            router,
        }
    }

    fn restart(&mut self) {
        drop(std::mem::replace(&mut self.router, Router::new()));
        self.router = app_with_config(self.config.clone());
    }
}

async fn data(router: &Router, method: Method, uri: &str, body: Option<Value>) -> Value {
    let response = authenticated_request(router, method, uri, body).await;
    let status = response.status();
    let payload = response_json(response).await;
    assert!(status.is_success(), "{uri}: {status}: {payload}");
    assert!(
        matches!(payload["code"].as_str(), Some("SUCCESS" | "0")),
        "{uri}: {payload}"
    );
    payload["data"].clone()
}

async fn dudes(router: &Router) -> Vec<Value> {
    data(router, Method::GET, "/api/v1/dudes", None)
        .await
        .as_array()
        .expect("Dude list")
        .clone()
}

#[tokio::test]
async fn default_dudes_have_independent_definitions_and_survive_restart() {
    let mut fixture = Fixture::new();
    let initial = dudes(&fixture.router).await;
    assert!(
        initial
            .iter()
            .any(|d| d["kind"] == "Memory" && d["memoryAutoApprove"] == false)
    );
    let main = initial
        .iter()
        .find(|d| d["isMain"] == true)
        .expect("platform main Dude");
    let main_code = main["dudeCode"]
        .as_str()
        .expect("main Dude code")
        .to_owned();
    let main_prompt = main["rolePrompt"].clone();
    assert!(
        main_prompt.is_string(),
        "Dude owns a complete agent definition"
    );
    let changed_prompt = "A deliberately independent ordinary-agent prompt.";

    data(
        &fixture.router,
        Method::PATCH,
        "/api/v1/agents/registry/internal/main-agent/definition",
        Some(json!({"rolePrompt": changed_prompt})),
    )
    .await;
    fixture.restart();

    let restored = dudes(&fixture.router).await;
    let restored_main = restored
        .iter()
        .find(|d| d["dudeCode"] == main_code)
        .expect("restored main Dude");
    assert_eq!(
        restored_main["rolePrompt"], main_prompt,
        "ordinary Agent edits must not silently rewrite independent Dude configuration"
    );
    assert_ne!(restored_main["rolePrompt"], changed_prompt);
    assert_eq!(restored.iter().filter(|d| d["isMain"] == true).count(), 1);
    assert_eq!(restored.iter().filter(|d| d["kind"] == "Memory").count(), 1);
}

#[tokio::test]
async fn robot_entry_resolves_one_platform_session_across_workspace_inputs_and_restart() {
    let mut fixture = Fixture::new();
    let first = data(
        &fixture.router,
        Method::POST,
        "/api/v1/dudes/main/session",
        Some(json!({"workspaceCode":"TRON_CORE"})),
    )
    .await;
    let second = data(
        &fixture.router,
        Method::POST,
        "/api/v1/dudes/main/session",
        Some(json!({"workspaceCode":"COMPLIANCE"})),
    )
    .await;
    assert_eq!(first["sessionCode"], second["sessionCode"]);
    assert!(first["sessionCode"].is_string());
    assert!(
        first["workspaceCode"].is_null(),
        "platform session must not acquire the current page's workspace"
    );
    fixture.restart();
    let restored = data(
        &fixture.router,
        Method::POST,
        "/api/v1/dudes/main/session",
        None,
    )
    .await;
    assert_eq!(restored["sessionCode"], first["sessionCode"]);

    for workspace in ["TRON_CORE", "COMPLIANCE"] {
        let groups = data(
            &fixture.router,
            Method::GET,
            &format!("/api/v1/chats/session-groups?workspaceCode={workspace}"),
            None,
        )
        .await;
        let listed: Vec<_> = groups
            .as_array()
            .expect("ordinary session groups")
            .iter()
            .flat_map(|group| group["sessions"].as_array().into_iter().flatten())
            .collect();
        assert!(
            listed
                .iter()
                .all(|session| session["sessionCode"] != first["sessionCode"]),
            "Dude interaction must remain outside user chat lists"
        );
    }
}

#[tokio::test]
async fn changing_main_dude_keeps_each_dudes_existing_conversation_and_blocks_main_deletion() {
    let fixture = Fixture::new();
    let initial = dudes(&fixture.router).await;
    let old_main = initial.iter().find(|d| d["isMain"] == true).unwrap();
    let old_code = old_main["dudeCode"].as_str().unwrap();
    let original = data(
        &fixture.router,
        Method::POST,
        "/api/v1/dudes/main/session",
        None,
    )
    .await;
    data(
        &fixture.router,
        Method::POST,
        "/api/v1/dudes",
        Some(json!({
            "dudeCode":"acceptance-brother", "name":"Acceptance Brother",
            "rolePrompt":"Independent acceptance Dude.", "loopCode":old_main["loopCode"],
            "avatarId":old_main["avatarId"], "candidateModelCodes":[],
            "enabled":false, "heartbeatMinutes":30, "memoryAutoApprove":false
        })),
    )
    .await;
    data(
        &fixture.router,
        Method::PUT,
        "/api/v1/dudes/acceptance-brother/main",
        None,
    )
    .await;
    let next = data(
        &fixture.router,
        Method::POST,
        "/api/v1/dudes/main/session",
        None,
    )
    .await;
    assert_ne!(original["sessionCode"], next["sessionCode"]);
    let old_session = data(
        &fixture.router,
        Method::POST,
        &format!("/api/v1/dudes/{old_code}/sessions"),
        None,
    )
    .await;
    assert_eq!(old_session["sessionCode"], original["sessionCode"]);

    let delete = authenticated_request(
        &fixture.router,
        Method::DELETE,
        "/api/v1/dudes/acceptance-brother",
        None,
    )
    .await;
    assert!(
        !delete.status().is_success(),
        "current main Dude must retain a resolvable robot entry"
    );
    assert_ne!(delete.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let listed = dudes(&fixture.router).await;
    assert_eq!(listed.iter().filter(|d| d["isMain"] == true).count(), 1);
}
