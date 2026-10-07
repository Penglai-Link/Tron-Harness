mod common;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use tron_backend::{application::agent::FrameworkHost, config::AppConfig};

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let response = common::authenticated_request(app, method, path, body).await;
    let status = response.status();
    (status, common::response_json(response).await)
}
async fn create(app: &axum::Router, workspace: &str) -> String {
    let (status, value) = call(
        app,
        Method::POST,
        &format!("/api/v1/chats/sessions?workspaceCode={workspace}"),
        Some(json!({"workspaceCode":workspace,"title":"native session"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{value}");
    value["data"]["sessionCode"].as_str().unwrap().into()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn in_memory_application_shares_native_and_business_storage_and_isolates_instances() {
    use std::sync::Arc;
    use tron_backend::{
        app::AppState,
        application::{chat::CreateChatSessionCommand, profile::ProfileRegistryService},
        config::{ResourceConfig, VariableCryptoConfig},
        shared::id::FixedSnowflakeIdProvider,
    };
    let directory = tempfile::tempdir().unwrap();
    let mut states = vec![];
    for index in 0..2 {
        let mut config = AppConfig::default();
        config.database_url = ":memory:".into();
        config.graph_storage_path = directory
            .path()
            .join(format!("graph-{index}.grafeo"))
            .to_string_lossy()
            .into_owned();
        config.skills_registry_root = directory
            .path()
            .join(format!("skills-{index}"))
            .to_string_lossy()
            .into_owned();
        config.resource = ResourceConfig::with_default_root(
            directory
                .path()
                .join(format!("resources-{index}"))
                .to_string_lossy()
                .into_owned(),
        );
        config.variable_crypto = VariableCryptoConfig::test_default();
        config.system_agents.codex_command.clear();
        config.system_agents.opencode_command.clear();
        let state = AppState::new_with_profiles(
            config,
            Arc::new(FixedSnowflakeIdProvider::new(900_000 + index)),
            ProfileRegistryService::new(":memory:").unwrap(),
        )
        .unwrap();
        assert_ne!(state.config.database_url, ":memory:");
        states.push(state);
    }
    assert_ne!(states[0].config.database_url, states[1].config.database_url);
    let session = states[0]
        .chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("isolated native session".into()),
                app_code: None,
                task_code: Some("task-fixture".into()),
                source_type: Some(tron_backend::domain::chat::ChatSessionSourceType::Task),
                source_code: Some("task-fixture".into()),
                agent_code: None,
            },
            None,
        )
        .unwrap();
    let framework = states[0].agent_framework.storage_framework().await.unwrap();
    assert_eq!(
        framework
            .sessions()
            .unwrap()
            .get(&session.session_code)
            .await
            .unwrap()
            .title,
        session.title
    );
    let id = session.session_code.clone();
    let database = states[0].agent_framework.shared_database().await.unwrap();
    let workspace: String = database
        .call(move |c| {
            c.query_row(
                "SELECT workspace_code FROM workspace_session_links WHERE session_id=?1",
                [&id],
                |row| row.get(0),
            )
        })
        .await
        .unwrap();
    assert_eq!(workspace, "TRON_CORE");
    let other = states[1].agent_framework.storage_framework().await.unwrap();
    assert!(matches!(
        other.sessions().unwrap().get(&session.session_code).await,
        Err(eliteforge_agent_core::StateError::NotFound)
    ));
    for state in &states {
        state.agent_framework.close().await;
    }
}
#[tokio::test]
async fn fresh_database_archive_admin_and_workspace_isolation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tron.sqlite3");
    let mut config = AppConfig::default();
    config.database_url = path.to_string_lossy().into_owned();
    config.system_agents.codex_command.clear();
    config.system_agents.opencode_command.clear();
    let app = common::app_with_config(config);
    let first = create(&app, "TRON_CORE").await;
    let second = create(&app, "COMPLIANCE").await;
    // Every normal operation checks the workspace even when identities are known.
    for (method, tail, body) in [
        (Method::GET, format!("{first}/messages"), None),
        (Method::GET, format!("{first}/user-input-requests"), None),
        (Method::POST, format!("{first}/archive"), None),
        (
            Method::PATCH,
            first.clone(),
            Some(json!({"title":"unauthorized"})),
        ),
        (Method::GET, format!("get-goal?sessionCode={first}"), None),
        (
            Method::POST,
            "stop-run".into(),
            Some(json!({"sessionCode":first})),
        ),
    ] {
        let separator = if tail.contains('?') { "&" } else { "?" };
        let (status, _) = call(
            &app,
            method,
            &format!("/api/v1/chats/sessions/{tail}{separator}workspaceCode=COMPLIANCE"),
            body,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{tail}");
    }
    let host = FrameworkHost::persistent(path.to_str().unwrap(), None);
    let framework = host.storage_framework().await.unwrap();
    let goals = framework.goals().unwrap();
    let goal = goals
        .set(eliteforge_agent_core::GoalRecord::new(
            &first,
            "Archive paused goal",
        ))
        .await
        .unwrap();
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{first}/archive?workspaceCode=TRON_CORE"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "active goal must prevent archive"
    );
    let paused = goals
        .transition(
            &goal.id,
            goal.revision,
            eliteforge_agent_core::GoalStatus::Paused,
            Some("user paused".into()),
        )
        .await
        .unwrap();
    assert_eq!(paused.status, eliteforge_agent_core::GoalStatus::Paused);
    for (id, workspace) in [(&first, "TRON_CORE"), (&second, "COMPLIANCE")] {
        let (status, value) = call(
            &app,
            Method::POST,
            &format!("/api/v1/chats/sessions/{id}/archive?workspaceCode={workspace}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{value}");
    }
    let (_, list) = call(&app, Method::GET, "/api/v1/admin/archived-sessions", None).await;
    assert_eq!(list["data"]["total"], 2);
    let (_, filtered) = call(
        &app,
        Method::GET,
        "/api/v1/admin/archived-sessions?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(filtered["data"]["total"], 1);
    let (_, detail) = call(
        &app,
        Method::GET,
        &format!("/api/v1/admin/archived-sessions/{first}"),
        None,
    )
    .await;
    assert_eq!(detail["data"]["session"]["workspaceCode"], "TRON_CORE");
    assert_eq!(detail["data"]["goals"][0]["status"], "PAUSED");
    assert!(
        detail["data"]["goalEvents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["goalCode"] == goal.id && e["kind"] == "GOAL_STATE_CHANGED")
    );
    assert_eq!(
        goals.get(&goal.id).await.unwrap().revision,
        paused.revision,
        "read-only archive detail must not mutate native goal"
    );
    let (status, restored) = call(
        &app,
        Method::POST,
        &format!("/api/v1/admin/archived-sessions/{first}/restore"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    assert_eq!(restored["data"]["workspaceCode"], "TRON_CORE");
    let (_, deleted) = call(
        &app,
        Method::POST,
        "/api/v1/admin/archived-sessions/batch-delete",
        Some(json!({"sessionCodes":[first,second]})),
    )
    .await;
    assert_eq!(deleted["data"]["succeeded"], 1, "{deleted}");
    assert_eq!(deleted["data"]["failed"], 1);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/admin/archived-sessions/{second}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let host = FrameworkHost::persistent(path.to_str().unwrap(), None);
    let database = host.shared_database().await.unwrap();
    let legacy:Vec<String>=database.call(|c| {
        let mut q=c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND (name IN ('chat_sessions','chat_messages','agent_run_history','conversation_runtime_sessions','agent_runtime_event_replay') OR name LIKE 'short_term_memory_%')")?;
        q.query_map([],|r|r.get(0))?.collect::<Result<Vec<_>,_>>()
    }).await.unwrap();
    assert!(legacy.is_empty(), "obsolete tables: {legacy:?}");
    host.close().await;
}
