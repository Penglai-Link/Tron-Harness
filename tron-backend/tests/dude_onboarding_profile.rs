//! Real Framework form interruption, durable resume, tool execution and scope regression.
mod common;
#[allow(dead_code)]
#[path = "support/candidate_model_fixture.rs"]
mod models;
use eliteforge_agent_core as fw;
use serde_json::{Value, json};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tron_backend::{
    app::AppState,
    application::agent::{
        AgentInvocationCommand, AgentInvocationService, ModelClient, ModelStreamSink,
    },
    config::{AppConfig, ResourceConfig},
    domain::{
        agent::{AgentCollaborationOwner, AgentModelConfig, ModelRequest, ModelResponse},
        dude::SoulState,
    },
    shared::event_bus::EventBusHandle,
};

fn form() -> Value {
    json!({"mode":"SubmitOnce","responseSchema":{"type":"object","required":["name","personality","customPersonality"],"additionalProperties":false,"properties":{"name":{"type":"string","minLength":1,"maxLength":100},"personality":{"type":"string"},"customPersonality":{"type":"string"}},"anyOf":[{"properties":{"personality":{"minLength":1}}},{"properties":{"customPersonality":{"minLength":1}}}]},"messages":[
        {"version":"v0.9.1","createSurface":{"surfaceId":"dude-soul-setup","catalogId":"https://eliteforge.dev/a2ui/v0.9.1/agent"}},
        {"version":"v0.9.1","updateComponents":{"surfaceId":"dude-soul-setup","components":[
            {"id":"root","component":"Column","children":["name","personality","custom","submit"]},
            {"id":"name","component":"TextField","label":"Name your Dude","value":{"path":"/response/name"}},
            {"id":"personality","component":"Select","label":"Personality","value":{"path":"/response/personality"},"options":[{"label":"Dry wit — playful observations, understated warmth","value":"wit"},{"label":"Quiet curiosity — attentive, perceptive, thoughtful","value":"quiet"},{"label":"Warm independence — caring and willing to disagree","value":"warm"},{"label":"Bright energy — expressive, lively and candid","value":"bright"}]},
            {"id":"custom","component":"TextField","label":"Custom personality","value":{"path":"/response/customPersonality"}},
            {"id":"submit","component":"Button","child":"submit-label","action":{"event":{"name":"submit","context":{"response":{"path":"/response"}}}}},
            {"id":"submit-label","component":"Text","text":"Confirm"}]}},
        {"version":"v0.9.1","updateDataModel":{"surfaceId":"dude-soul-setup","value":{"response":{"name":"","personality":"","customPersonality":""}}}}]})
}

#[derive(Default)]
struct Model {
    calls: AtomicUsize,
    requests: Mutex<Vec<fw::ChatRequest>>,
}
impl ModelClient for Model {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("native path required".into()) })
    }
    fn generate_native_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            assert!(
                config.thinking_enabled,
                "initialization must retain thinking"
            );
            self.requests.lock().unwrap().push(request.clone());
            let step = self.calls.fetch_add(1, Ordering::SeqCst);
            if step < 4 {
                let mut names = request
                    .tools
                    .iter()
                    .map(|t| t.call_name.as_str())
                    .collect::<Vec<_>>();
                names.sort();
                assert_eq!(
                    names,
                    ["a2ui_request_input", "dude_soul_read", "dude_soul_update"]
                );
            }
            Ok(match step {
                0 => ModelResponse::tool_call("a2ui_request_input", form()),
                1 => ModelResponse::tool_call("dude_soul_read", json!({})),
                2 => {
                    assert!(
                        request
                            .messages
                            .iter()
                            .any(|m| m.content.protocol_text().contains("River")),
                        "submitted name reaches the model"
                    );
                    ModelResponse::tool_call(
                        "dude_soul_update",
                        json!({"name":"River","initialSoul":"# Personality\nQuiet, curious and observant.\n\n# Expression\nThoughtful, warm responses."}),
                    )
                }
                3 => ModelResponse::final_output("Saved River"),
                _ => ModelResponse::final_output("Ordinary task complete"),
            })
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

fn state(config: AppConfig) -> AppState {
    // No always-on HTTP dispatcher: closing this fixture must release the graph
    // as a real process exit would before opening the same database again.
    static IDS: AtomicUsize = AtomicUsize::new(0);
    let profiles =
        tron_backend::application::profile::ProfileRegistryService::new(":memory:").unwrap();
    profiles
        .register_or_update("USR-AUTH-0001", "profile.operator")
        .unwrap();
    let state = AppState::new_with_profiles(
        config,
        Arc::new(tron_backend::shared::id::FixedSnowflakeIdProvider::new(
            800_000_000_000_000_000 + IDS.fetch_add(1, Ordering::SeqCst) as i64 * 1_000_000,
        )),
        profiles,
    )
    .unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    models::seed_loop(&state);
    state
}
fn service(state: &AppState, model: Arc<Model>, session: &str) -> AgentInvocationService {
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
        code: "main-dude".into(),
        origin_session_code: Some(session.into()),
        task_code: None,
    })
}
fn command(session: &str, resume: Value) -> AgentInvocationCommand {
    let mut command: AgentInvocationCommand = serde_json::from_value(json!({"prompt":"Wake up","workspaceCode":null,"conversationCode":session,"agentCode":"main-dude","model":{"modelCode":"fixture","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture","thinkingEnabled":true,"maxTokens":4096,"contextWindowTokens":32000,"maxRetries":0}})).unwrap();
    command.supports_user_input_requests = true;
    command.transcript_input_kind = Some("USER_PROMPT".into());
    command.transcript_input_metadata = Some(
        json!({"onboarding":true,"resume":resume,"a2uiClientCapabilities":{"version":"v0.9.1","catalogIds":["https://eliteforge.dev/a2ui/v0.9.1/agent"]}}),
    );
    command
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn awakening_api_carries_negotiated_form_capabilities_into_the_queued_wakeup() {
    let mut captured = None;
    let app = common::app_with_state_setup(common::prepared_config(AppConfig::default()), |s| {
        captured = Some(s.clone())
    });
    let state = captured.unwrap();
    let token = common::authenticated_bearer_token(&app).await;
    let capabilities =
        json!({"version":"v0.9.1","catalogIds":["https://eliteforge.dev/a2ui/v0.9.1/agent"]});
    let response = common::request_with_headers(
        &app,
        axum::http::Method::POST,
        "/api/v1/dudes/main/awakening",
        Some(json!({"a2ui":capabilities})),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let event = state
        .dudes
        .events("main-dude")
        .unwrap()
        .into_iter()
        .find(|e| e.dedup_key.starts_with("soul-awakening:"))
        .unwrap();
    assert_eq!(event.payload["a2ui"], capabilities);
    assert_eq!(event.payload["onboarding"], true);
    assert!(state.agent_framework.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn onboarding_form_resumes_after_restart_with_only_soul_tools_and_preserves_thinking() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::prepared_config(AppConfig {
        database_url: dir.path().join("tron.sqlite3").display().to_string(),
        graph_storage_path: dir.path().join("tron.grafeo").display().to_string(),
        skills_registry_root: dir.path().join("skills").display().to_string(),
        resource: ResourceConfig::with_default_root(
            dir.path().join("resources").display().to_string(),
        ),
        ..Default::default()
    });
    let first = state(config.clone());
    let (soul, _) = first
        .dudes
        .begin_awakening("main-dude", "USR-AUTH-0001")
        .unwrap();
    let session = soul.session_code.unwrap();
    first
        .chats
        .ensure_dude_session(
            &first.dudes.session(&session).unwrap().unwrap(),
            Some("USR-AUTH-0001".into()),
        )
        .unwrap();
    let ordinary_model = Arc::new(Model {
        calls: AtomicUsize::new(4),
        ..Default::default()
    });
    let mut ordinary = command(&session, json!([]));
    ordinary.prompt = Some("Help with my task before initialization".into());
    ordinary.transcript_input_metadata = Some(json!({}));
    service(&first, ordinary_model.clone(), &session)
        .invoke(ordinary, Some("USR-AUTH-0001".into()), Some("en-US".into()))
        .await
        .unwrap();
    assert!(
        ordinary_model.requests.lock().unwrap()[0]
            .tools
            .iter()
            .any(|t| t.call_name == "agent_spawn")
    );
    assert_eq!(
        first.dudes.soul("main-dude").unwrap().state,
        SoulState::Uninitialized
    );
    let model = Arc::new(Model::default());
    let result = service(&first, model.clone(), &session)
        .invoke(
            command(&session, json!([])),
            Some("USR-AUTH-0001".into()),
            Some("en-US".into()),
        )
        .await
        .unwrap();
    assert_eq!(
        result.native.as_ref().unwrap().status,
        tron_backend::domain::agent::AgentNativeRunStatus::AwaitingInput
    );
    let pending = first
        .agent_framework
        .pending_native_interactions(&session)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].request.presentation.as_ref().unwrap()["kind"],
        "a2ui"
    );
    assert_eq!(
        first.dudes.soul("main-dude").unwrap().state,
        SoulState::AwaitingUser
    );
    let interaction = pending[0].request.id.to_string();
    assert!(first.agent_framework.close().await.is_success());
    drop(first);

    let reopened = state(config);
    assert_eq!(
        reopened
            .agent_framework
            .pending_native_interactions(&session)
            .await
            .unwrap()
            .len(),
        1
    );
    let mut answer = command(
        &session,
        json!([{"interruptId":interaction,"status":"resolved","payload":{"name":"River","personality":"quiet","customPersonality":""}}]),
    );
    answer.prompt = Some("River; quiet curiosity".into());
    let saved = service(&reopened, model.clone(), &session)
        .invoke(answer, Some("USR-AUTH-0001".into()), Some("en-US".into()))
        .await
        .unwrap();
    assert_eq!(saved.final_output, "Saved River");
    let soul = reopened.dudes.soul("main-dude").unwrap();
    assert_eq!(soul.state, SoulState::Ready);
    assert_eq!(soul.name, "River");
    assert!(soul.markdown.contains("Quiet"));
    for request in model.requests.lock().unwrap().iter() {
        let mut names = request
            .tools
            .iter()
            .map(|t| t.call_name.as_str())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            ["a2ui_request_input", "dude_soul_read", "dude_soul_update"]
        );
        let text = request
            .messages
            .iter()
            .filter(|m| m.role == fw::Role::System)
            .map(|m| m.content.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("Current authorized tool domains"));
        assert!(!text.contains("available_skills"));
        assert!(!text.contains("工作与通知协议"));
        assert!(
            text.len() < 6000,
            "bounded initialization instructions: {}",
            text.len()
        );
    }
    let mut ordinary = command(&session, json!([]));
    ordinary.prompt = Some("Help with a real task".into());
    ordinary.transcript_input_metadata = Some(json!({}));
    service(&reopened, model.clone(), &session)
        .invoke(ordinary, Some("USR-AUTH-0001".into()), Some("en-US".into()))
        .await
        .unwrap();
    assert!(
        model
            .requests
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .tools
            .iter()
            .any(|t| t.call_name == "agent_spawn"),
        "ordinary Dude chat keeps collaboration tools"
    );
    assert!(reopened.agent_framework.close().await.is_success());
}
