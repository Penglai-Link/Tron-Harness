//! Real Framework execution with platform scope and controlled model latency.
mod common;
#[path = "support/candidate_model_fixture.rs"]
mod models;

use eliteforge_agent_core as fw;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tron_backend::{
    app::AppState,
    application::agent::{
        AgentInvocationCommand, AgentInvocationService, CreateWorkspaceInternalAgentCommand,
        ModelClient, ModelStreamSink,
    },
    config::{AppConfig, ResourceConfig},
    domain::agent::{AgentCollaborationOwner, AgentModelConfig, ModelRequest, ModelResponse},
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
                .find(|message| message.role == fw::Role::User)
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

#[tokio::test]
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
    let _router = common::app_with_state_setup(config, |state| captured = Some(state.clone()));
    let state = captured.unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    models::seed_loop(&state);
    state
        .agent_management
        .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
            agent_code: "acceptance-dude".into(),
            name: "Acceptance Dude".into(),
            role_prompt: "Run a focused acceptance task.".into(),
            loop_code: Some("test-react".into()),
            avatar_id: Some("robot".into()),
            group_name: None,
            candidate_model_codes: vec![],
        })
        .unwrap();

    let model = Arc::new(ControlledModel::default());
    let foreground_service = service(&state, model.clone(), "acceptance-interactive");
    let task_service = service(&state, model.clone(), "acceptance-task");
    let foreground = tokio::spawn(async move {
        foreground_service
            .invoke(
                command("acceptance-interactive", "foreground-hold"),
                Some("USR-AUTH-0001".into()),
                Some("en-US".into()),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(10), model.foreground_started.notified())
        .await
        .expect("foreground reached the real model with platform STM");
    let background = tokio::time::timeout(
        Duration::from_secs(10),
        task_service.invoke(
            command("acceptance-task", "background-work"),
            Some("USR-AUTH-0001".into()),
            Some("en-US".into()),
        ),
    )
    .await
    .expect("background must not wait for foreground model completion")
    .expect("platform background run succeeds");
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
            history
                .records
                .messages
                .iter()
                .any(|message| {
                    message.content.iter().any(|part|
            matches!(part, fw::HistoryContentBlock::Text { text } if text == expected))
                }),
            "{session} persists its own result"
        );
    }
    assert!(state.agent_framework.close().await.is_success());
}
