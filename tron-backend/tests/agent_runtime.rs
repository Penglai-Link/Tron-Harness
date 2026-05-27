use std::{sync::Arc, time::Duration};

use serde_json::json;
use tron_backend::{
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable,
        InProcessTool, ModelClient, ModelRequest, ModelResponse, ReactAgentBuilder, ToolError,
        ToolObservation, ToolRegistry, ToolRequest,
    },
    shared::{
        event_bus::{BusRouteClass, EventBusConfig, EventBusRuntimeBuilder, RecordingEventSink},
        id::FixedSnowflakeIdProvider,
    },
};

#[derive(Clone)]
struct ScriptedModelClient {
    responses: Arc<std::sync::Mutex<Vec<ModelResponse>>>,
}

impl ScriptedModelClient {
    fn new(responses: Vec<ModelResponse>) -> Self {
        Self {
            responses: Arc::new(std::sync::Mutex::new(responses)),
        }
    }
}

impl ModelClient for ScriptedModelClient {
    fn generate<'a>(
        &'a self,
        _config: &'a AgentModelConfig,
        _request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut guard = self
                .responses
                .lock()
                .expect("scripted model mutex poisoned");
            if guard.is_empty() {
                return Err("no scripted response".to_string());
            }
            Ok(guard.remove(0))
        })
    }
}

fn base_context(run_code: &str) -> AgentRunContext {
    AgentRunContext {
        conversation_code: Some("CONV-1".into()),
        run_code: run_code.into(),
        workspace_code: "TRON_CORE".into(),
        user_code: Some("USR-1".into()),
        request_id: Some("REQ-1".into()),
        stream_key: Some("CONV-1".into()),
        agent_kind: AgentKind::React,
        model: AgentModelConfig {
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "gpt-test".into(),
            temperature: Some(0.2),
            max_tokens: Some(128),
            top_p: Some(1.0),
            timeout_ms: Some(1000),
            max_retries: Some(0),
        },
        variables: vec![EffectiveVariable {
            key: "REGION".into(),
            value: "us".into(),
            source: "LOCAL".into(),
        }],
        skills: vec![EffectiveSkill {
            skill_name: "planner".into(),
            description: "Plan work".into(),
            instruction: Some("Think briefly.".into()),
            tool_names: vec!["echo".into()],
        }],
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            description: "Echo input".into(),
            input_schema: json!({"type": "object"}),
            skill_name: Some("planner".into()),
            required_variable_keys: vec![],
        }],
        execution_scope: AgentExecutionScope {
            app_code: Some("APP-1".into()),
            repository_url: Some("https://example.test/repo.git".into()),
            local_directory: Some("/workspace/repo".into()),
            branch_name: Some("main".into()),
            task_code: Some("TASK-1".into()),
        },
        limits: AgentRunLimits {
            max_steps: 4,
            max_tool_calls: 2,
        },
        messages: vec![AgentMessage::user("hello")],
        observations: Vec::new(),
        started_at: "2026-05-27T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn react_agent_completes_final_answer_and_publishes_metadata_only_events() {
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(100)),
    )
    .register_sink(sink.clone())
    .build();
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "done",
        )])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("RUN-FINAL"))
        .await
        .expect("final answer run should pass");

    assert_eq!(result.final_output, "done");
    assert_eq!(result.step_count, 1);
    assert!(sink.wait_for_len(5, Duration::from_secs(2)));
    let events = sink.events();
    let names: Vec<_> = events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "agent.run.started",
            "agent.model.requested",
            "agent.model.responded",
            "agent.output.created",
            "agent.run.completed",
        ]
    );
    for event in events {
        let serialized = format!("{:?}", event);
        assert!(!serialized.contains("hello"));
        assert!(!serialized.contains("done"));
        assert!(!serialized.contains("sk-test"));
        assert!(!serialized.contains("prompt"));
        assert!(!serialized.contains("completion"));
    }
}

#[tokio::test]
async fn react_agent_executes_allowed_tool_and_keeps_runs_isolated() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            },
        )))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"value": "a"})),
            ModelResponse::final_output("after-tool"),
            ModelResponse::final_output("second-run"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let first = agent
        .run(base_context("RUN-TOOL"))
        .await
        .expect("tool run should pass");
    let second = agent
        .run(base_context("RUN-SECOND"))
        .await
        .expect("second run should pass");

    assert_eq!(first.final_output, "after-tool");
    assert_eq!(first.observations.len(), 1);
    assert_eq!(second.final_output, "second-run");
    assert!(second.observations.is_empty());
}

#[tokio::test]
async fn tool_registry_rejects_missing_or_unauthorized_tools() {
    let registry = ToolRegistry::default();
    let context = base_context("RUN-DENIED");

    let missing = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "echo".into(),
                input: json!({}),
            },
            &context,
        )
        .expect_err("registered executor should be required");
    assert!(matches!(missing, ToolError::MissingTool(_)));

    registry
        .register(Arc::new(InProcessTool::new("hidden", |_request| {
            Ok(ToolObservation {
                tool_name: "hidden".into(),
                output: "hidden".into(),
            })
        })))
        .expect("hidden tool should register");
    let unauthorized = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "hidden".into(),
                input: json!({}),
            },
            &context,
        )
        .expect_err("tool must be in allowed snapshot");
    assert!(matches!(unauthorized, ToolError::UnauthorizedTool(_)));
}

#[tokio::test]
async fn model_request_builders_use_frozen_model_config() {
    let config = base_context("RUN-MODEL").model;
    let request = ModelRequest {
        messages: vec![AgentMessage::user("hi")],
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            description: "Echo".into(),
            input_schema: json!({"type": "object"}),
            skill_name: None,
            required_variable_keys: vec![],
        }],
    };

    let chat = config.chat_completions_request(&request);
    assert_eq!(chat["model"], json!("gpt-test"));
    assert_eq!(chat["tools"][0]["function"]["name"], json!("echo"));

    let responses = config.responses_request(&request);
    assert_eq!(responses["model"], json!("gpt-test"));
    assert_eq!(responses["input"][0]["content"], json!("hi"));

    let embeddings = config.embeddings_request("embed me");
    assert_eq!(embeddings["model"], json!("gpt-test"));
    assert_eq!(embeddings["input"], json!("embed me"));
}

#[tokio::test]
async fn max_step_exhaustion_fails_without_output_success() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
        },
        ..base_context("RUN-LIMIT")
    };
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("echo", |_request| {
            Ok(ToolObservation {
                tool_name: "echo".into(),
                output: "again".into(),
            })
        })))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::tool_call(
            "echo",
            json!({}),
        )])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(context)
        .await
        .expect_err("tool loop should exhaust max steps");

    assert_eq!(error.to_string(), "agent run exceeded max steps");
}
