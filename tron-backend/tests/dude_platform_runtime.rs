//! Real Framework execution with platform scope and controlled model latency.
mod common;
#[path = "support/candidate_model_fixture.rs"]
mod models;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use eliteforge_agent_core as fw;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tower::ServiceExt;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteDudeRepository,
    app::AppState,
    application::agent::{
        AgentInvocationCommand, AgentInvocationService, CreateWorkspaceInternalAgentCommand,
        ModelClient, ModelStreamSink,
    },
    application::{dude::DudeRepository, dude_memory::DudeMemorySourceCollector},
    config::{AppConfig, ResourceConfig},
    domain::agent::{AgentCollaborationOwner, AgentModelConfig, ModelRequest, ModelResponse},
    domain::dude::{Dude, DudeKind, DudeSession, DudeSessionPurpose},
    shared::event_bus::EventBusHandle,
};

#[derive(Default)]
struct ControlledModel {
    foreground_started: Notify,
    release_foreground: Notify,
    requests: Mutex<Vec<fw::ChatRequest>>,
}

impl ModelClient for ControlledModel {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Ok(ModelResponse::final_output("summary")) })
    }

    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.clone());
            let current = request
                .messages
                .iter()
                .rev()
                .find(|message| {
                    message.role == fw::Role::User
                        && (message.content.text().contains("foreground-hold")
                            || message.content.text().contains("background-work"))
                })
                .map(|message| message.content.text())
                .unwrap_or_default();
            if current.contains("foreground-hold") {
                self.foreground_started.notify_one();
                self.release_foreground.notified().await;
            }
            Ok(ModelResponse::final_output(
                if current.contains("background-work") {
                    "background-completed-before-foreground"
                } else {
                    "foreground-completed"
                },
            ))
        })
    }

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}

fn command(session: &str, prompt: &str) -> AgentInvocationCommand {
    serde_json::from_value(json!({
        "prompt":prompt, "workspaceCode":null, "conversationCode":session,
        "agentCode":"acceptance-dude", "model": {
            "modelCode":"acceptance-platform-model", "protocol":"CHAT_COMPLETIONS",
            "baseUrl":"http://127.0.0.1:1/v1", "apiKey":"synthetic", "modelName":"acceptance",
            "maxTokens":2048, "contextWindowTokens":32000, "maxRetries":0
        }
    }))
    .unwrap()
}

fn service(state: &AppState, model: Arc<ControlledModel>, session: &str) -> AgentInvocationService {
    AgentInvocationService::new(
        state.workspaces.clone(),
        state.apps.clone(),
        state.variables.clone(),
        state.skills.clone(),
        state.agent_management.clone(),
        state.agent_runs.clone(),
        state.mcp.clone(),
        EventBusHandle::disabled(),
        model,
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    )
    .with_framework_host(state.agent_framework.clone())
    .with_short_term_memory(state.short_term_memory.clone())
    .with_prompt_service(state.prompts.clone())
    .with_active_runs(state.active_runs.clone())
    .with_platform_scope()
    .with_collaboration_owner(AgentCollaborationOwner {
        kind: "DUDE".into(),
        code: "acceptance-dude".into(),
        origin_session_code: Some(session.into()),
        task_code: None,
    })
}

// The application uses Tokio's multithread runtime: its synchronous SQLite
// bridge may block_in_place while an async Framework transaction owns the gate.
// A current_thread harness would block the sole executor, unlike production.
// Parallelism remains proven by the explicit held model call below, not by the
// worker count or by allowing the foreground response to finish early.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_short_term_memory_runs_background_while_foreground_is_still_answering() {
    let directory = tempfile::tempdir().unwrap();
    let config = common::prepared_config(AppConfig {
        database_url: directory
            .path()
            .join("runtime.sqlite3")
            .display()
            .to_string(),
        graph_storage_path: directory
            .path()
            .join("runtime.grafeo")
            .display()
            .to_string(),
        skills_registry_root: directory.path().join("skills").display().to_string(),
        resource: ResourceConfig::with_default_root(
            directory.path().join("resources").display().to_string(),
        ),
        ..AppConfig::default()
    });
    let mut captured = None;
    let router = common::app_with_state_setup(config, |state| captured = Some(state.clone()));
    let state = captured.unwrap();
    state.knowledge_bases.stop_ingestion_workers();

    state
        .dudes
        .create(tron_backend::application::dude::CreateDudeCommand {
            kind: DudeKind::Custom,
            enabled: true,
            heartbeat_minutes: 30,
            memory_auto_approve: false,
            definition: CreateWorkspaceInternalAgentCommand {
                description: String::new(),
                agent_code: "acceptance-dude".into(),
                name: "Acceptance Dude".into(),
                role_prompt: "Run a focused acceptance task.".into(),

                avatar_id: Some("robot".into()),
                group_name: None,
                candidate_model_codes: vec![],
            },
        })
        .unwrap();
    // Create the same durable product projection used by real Dude sessions.
    // In particular, DudeSession has no user field: the native chat link is the
    // authority that separates profiles during memory collection.
    let mut repository = SqliteDudeRepository::connect(&state.config.database_url).unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    repository
        .save(&Dude {
            dude_code: "acceptance-dude".into(),
            kind: DudeKind::Custom,
            enabled: true,
            heartbeat_minutes: 30,
            memory_auto_approve: false,
            is_main: false,
            // Keep unrelated scheduled heartbeat work outside this fixture's
            // execution window; epoch timestamps would make it immediately due.
            created_at: now,
            updated_at: now,
        })
        .unwrap();
    let mut other_owner = repository
        .list()
        .unwrap()
        .into_iter()
        .find(|d| d.dude_code == "acceptance-dude")
        .unwrap();
    other_owner.dude_code = "acceptance-other-dude".into();
    repository.save(&other_owner).unwrap();
    for (session, user) in [
        ("acceptance-interactive", "USR-AUTH-0001"),
        ("acceptance-task", "USR-AUTH-0001"),
        ("acceptance-foreign-profile", "USR-OTHER-PROFILE"),
    ] {
        let persisted = repository
            .resolve_session(&DudeSession {
                session_code: session.into(),
                dude_code: if session == "acceptance-foreign-profile" {
                    "acceptance-other-dude".into()
                } else {
                    "acceptance-dude".into()
                },
                // General background work needs no invented task identity.
                // Real task scheduling and bindings have separate API/browser
                // acceptance; Task purpose must refer to a persisted task.
                purpose: if session == "acceptance-interactive" {
                    DudeSessionPurpose::Interactive
                } else {
                    DudeSessionPurpose::Feedback
                },
                correlation_key: session.into(),
                title: session.into(),
                workspace_code: None,
                created_at: now,
                updated_at: now,
            })
            .unwrap();
        assert_eq!(persisted.session_code, session);
        state
            .chats
            .ensure_dude_session(&persisted, Some(user.into()))
            .unwrap();
    }

    let model = Arc::new(ControlledModel::default());
    let foreground_service = service(&state, model.clone(), "acceptance-interactive");
    let task_service = service(&state, model.clone(), "acceptance-task");
    let mut foreground = tokio::spawn(async move {
        foreground_service
            .invoke(
                command("acceptance-interactive", "foreground-hold"),
                Some("USR-AUTH-0001".into()),
                Some("en-US".into()),
            )
            .await
    });
    let foreground_started_at = std::time::Instant::now();
    tokio::select! {
        _ = model.foreground_started.notified() => {},
        completed = &mut foreground => {
            panic!("foreground ended before reaching the controlled model: {completed:?}");
        },
        _ = tokio::time::sleep(Duration::from_secs(60)) => {
            panic!("foreground did not reach the controlled model within 60s; invocation is still running");
        },
    }
    eprintln!(
        "foreground preparation elapsedMs={}",
        foreground_started_at.elapsed().as_millis()
    );
    let background_started_at = std::time::Instant::now();
    let background = tokio::time::timeout(
        Duration::from_secs(60),
        task_service.invoke(
            command("acceptance-task", "background-work"),
            Some("USR-AUTH-0001".into()),
            Some("en-US".into()),
        ),
    )
    .await
    .unwrap_or_else(|_| {
        let requests = model.requests.lock().unwrap();
        let seen = requests
            .iter()
            .enumerate()
            .map(|(index, request)| {
                let last_user = request
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.role == fw::Role::User)
                    .map(|message| message.content.text())
                    .unwrap_or_default();
                format!("request {index}: last user={last_user:?}")
            })
            .collect::<Vec<_>>();
        panic!(
            "background must not wait for foreground model completion; model dispatches={}\n{}",
            requests.len(),
            seen.join("\n")
        );
    })
    .expect("platform background run succeeds");
    eprintln!(
        "background preparation and completion elapsedMs={}",
        background_started_at.elapsed().as_millis()
    );
    assert_eq!(
        background.final_output,
        "background-completed-before-foreground"
    );
    assert!(
        !foreground.is_finished(),
        "foreground gate remains closed while background completes"
    );
    assert_eq!(
        background.workspace_code, "",
        "platform execution must not invent a workspace"
    );

    model.release_foreground.notify_one();
    let foreground = tokio::time::timeout(Duration::from_secs(10), foreground)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(foreground.final_output, "foreground-completed");
    let foreign = service(&state, model.clone(), "acceptance-foreign-profile")
        .with_collaboration_owner(AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "acceptance-other-dude".into(),
            origin_session_code: Some("acceptance-foreign-profile".into()),
            task_code: None,
        })
        .invoke(
            command("acceptance-foreign-profile", "foreign-profile-work"),
            Some("USR-OTHER-PROFILE".into()),
            Some("en-US".into()),
        )
        .await
        .unwrap();
    let token = common::authenticated_bearer_token(&router).await;
    for (uri, expected) in [
        (
            format!("/api/v1/agents/runs/{}", foreground.run_code),
            StatusCode::OK,
        ),
        (
            format!("/api/v1/agents/runs/{}", foreign.run_code),
            StatusCode::NOT_FOUND,
        ),
        (
            format!(
                "/api/v1/agents/runs/{}?sessionCode=acceptance-interactive",
                foreign.run_code
            ),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&uri)
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-dude-code", "acceptance-dude")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = common::response_json(response).await;
        assert_eq!(
            status, expected,
            "authoritative run ownership for {uri}: {body}"
        );
    }
    let framework = state.agent_framework.storage_framework().await.unwrap();
    for (session, expected) in [
        ("acceptance-interactive", "foreground-completed"),
        ("acceptance-task", "background-completed-before-foreground"),
    ] {
        let history = framework
            .history()
            .unwrap()
            .read_session(session, Default::default())
            .await
            .unwrap();
        assert!(
            history.records.messages.iter().any(|message| {
                message.content.iter().any(|part|
            matches!(part, fw::HistoryContentBlock::Text { text } if text == expected))
            }),
            "{session} persists its own result"
        );
    }

    let collector = DudeMemorySourceCollector::new(state.agent_framework.clone());
    let sources = collector
        .collect("USR-AUTH-0001", "separate-memory-dude")
        .await
        .unwrap();
    assert!(
        sources
            .iter()
            .any(|s| s.source_code.starts_with("acceptance-interactive/"))
    );
    assert!(
        sources
            .iter()
            .any(|s| s.source_code.starts_with("acceptance-task/"))
    );
    assert!(
        sources
            .iter()
            .all(|s| !s.source_code.starts_with("acceptance-foreign-profile/")),
        "platform Dude history must use trusted profile links even though DudeSession has no userCode"
    );
    let cursors = sources
        .iter()
        .map(|s| (s.identity(), s.version.clone()))
        .collect();
    assert!(
        collector
            .collect_incremental("USR-AUTH-0001", "separate-memory-dude", &cursors)
            .await
            .unwrap()
            .is_empty(),
        "unchanged persisted sources must not be analyzed twice"
    );
    assert!(
        collector
            .collect("USR-AUTH-0001", "acceptance-dude")
            .await
            .unwrap()
            .is_empty(),
        "a memory Dude must exclude all of its own native sessions"
    );
    assert!(state.agent_framework.close().await.is_success());
}
