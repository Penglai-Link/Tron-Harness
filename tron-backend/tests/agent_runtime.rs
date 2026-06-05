use std::{
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use serde_json::json;
use tron_backend::{
    adapters::outbound::model::{chat_completions_request, embeddings_request, responses_request},
    application::agent::{ModelClient, ReactAgentBuilder},
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable,
        InProcessTool, ModelRequest, ModelResponse, ToolCall, ToolError, ToolObservation,
        ToolRegistry, ToolRequest,
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
        locale: "en-US".into(),
        request_id: Some("REQ-1".into()),
        stream_key: Some("CONV-1".into()),
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: AgentModelConfig {
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "gpt-test".into(),
            thinking_enabled: false,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: Some(1.0),
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
            domain_code: None,
            description: "Echo input".into(),
            input_schema: json!({"type": "object"}),
            skill_name: Some("planner".into()),
            required_variable_keys: vec![],
        }],
        tool_exposure: Default::default(),
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
            max_parallel_tool_calls: 2,
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
    assert!(sink.wait_for_len(6, Duration::from_secs(2)));
    let events = sink.events();
    let names: Vec<_> = events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "agent.run.requested",
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
async fn react_agent_continues_after_tool_execution_failure() {
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-failure",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(101)),
    )
    .register_sink(sink.clone())
    .build();
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("flaky", |_request: ToolRequest| {
            Err(ToolError::ExecutionFailed("boom".into()))
        })))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("flaky", json!({"value": "x"})),
            ModelResponse::final_output("after-failure"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run({
            let mut context = base_context("RUN-TOOL-FAILURE");
            context.tools.push(EffectiveTool {
                tool_name: "flaky".into(),
                domain_code: None,
                description: "Flaky".into(),
                input_schema: json!({"type": "object"}),
                skill_name: Some("planner".into()),
                required_variable_keys: vec![],
            });
            context
        })
        .await
        .expect("tool execution failure should not abort the run");

    assert_eq!(result.final_output, "after-failure");
    assert_eq!(result.observations.len(), 1);
    assert_eq!(result.observations[0].tool_name, "flaky");
    assert_eq!(result.observations[0].output, "boom");
    assert!(sink.wait_for_len(12, Duration::from_secs(2)));
    let events = sink.events();
    assert!(events.iter().any(|event| event.event_name == "agent.tool.failed"));
    assert!(events.iter().any(|event| event.event_name == "agent.observation.created"));
    assert!(events.iter().any(|event| event.event_name == "agent.run.completed"));
    assert!(!events.iter().any(|event| event.event_name == "agent.run.failed"));
}

#[tokio::test]
async fn react_agent_executes_multiple_tool_calls_in_one_model_turn() {
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
            ModelResponse::tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("RUN-MULTI-TOOL"))
        .await
        .expect("multi tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(result.observations[1].output, "observed:\"b\"");
}

#[tokio::test]
async fn react_agent_executes_multiple_tool_calls_concurrently_by_default() {
    let registry = ToolRegistry::default();
    let barrier = Arc::new(Barrier::new(2));
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let barrier = Arc::clone(&barrier);
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            move |request: ToolRequest| {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                let mut observed = max_running.load(Ordering::SeqCst);
                while current > observed {
                    match max_running.compare_exchange(
                        observed,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
                barrier.wait();
                thread::sleep(Duration::from_millis(50));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = tokio::time::timeout(
        Duration::from_secs(2),
        agent.run(base_context("RUN-PARALLEL")),
    )
    .await
    .expect("parallel tool calls should complete within timeout")
    .expect("parallel tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(result.observations[1].output, "observed:\"b\"");
    assert_eq!(max_running.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn react_agent_executes_single_tool_call_without_parallel_batching() {
    let registry = ToolRegistry::default();
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            move |request: ToolRequest| {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                let mut observed = max_running.load(Ordering::SeqCst);
                while current > observed {
                    match max_running.compare_exchange(
                        observed,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
                thread::sleep(Duration::from_millis(50));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("RUN-SINGLE-TOOL"))
        .await
        .expect("single tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 1);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(max_running.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn react_agent_batches_parallel_tool_calls_by_parallel_limit() {
    let registry = ToolRegistry::default();
    let barrier = Arc::new(Barrier::new(2));
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let barrier = Arc::clone(&barrier);
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            move |request: ToolRequest| {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                let mut observed = max_running.load(Ordering::SeqCst);
                while current > observed {
                    match max_running.compare_exchange(
                        observed,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
                barrier.wait();
                thread::sleep(Duration::from_millis(20));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                },
                ToolCall {
                    call_id: Some("call-3".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "c"}),
                },
                ToolCall {
                    call_id: Some("call-4".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "d"}),
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("RUN-BATCHED-PARALLEL");
    context.limits.max_tool_calls = 6;
    context.limits.max_parallel_tool_calls = 2;
    let result = tokio::time::timeout(Duration::from_secs(2), agent.run(context))
        .await
        .expect("batched parallel tool calls should complete within timeout")
        .expect("batched parallel tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 4);
    assert_eq!(max_running.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn react_agent_executes_remaining_parallel_budget_before_failing() {
    let registry = ToolRegistry::default();
    let executed = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let executed = Arc::clone(&executed);
            move |request: ToolRequest| {
                executed.fetch_add(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::tool_calls(
            vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                },
                ToolCall {
                    call_id: Some("call-3".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "c"}),
                },
                ToolCall {
                    call_id: Some("call-4".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "d"}),
                },
            ],
        )])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("RUN-PARALLEL-BUDGET");
    context.limits.max_tool_calls = 3;
    context.limits.max_parallel_tool_calls = 2;
    let error = agent
        .run(context)
        .await
        .expect_err("run should fail after exhausting total tool-call budget");

    assert_eq!(error.to_string(), "agent run exceeded max tool calls");
    assert_eq!(executed.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn react_agent_publishes_tool_agui_correlation_payloads() {
    let long_command = "cargo test --package tron-backend --test agent_runtime ".repeat(3);
    let expected_args = serde_json::to_string(&json!({ "command": long_command.clone() }))
        .expect("tool args should serialize");
    let expected_result = format!("observed:\"{long_command}\"");
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["command"]),
                })
            },
        )))
        .expect("tool should register");
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-tool",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(200)),
    )
    .register_sink(sink.clone())
    .build();
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"command": long_command})),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    agent
        .run(base_context("RUN-TOOL-META"))
        .await
        .expect("tool run should pass");

    assert!(sink.wait_for_len(11, Duration::from_secs(2)));
    let events = sink.events();
    let tool_started = events
        .iter()
        .find(|event| event.event_name == "agent.tool.started")
        .expect("tool start event should be published");
    let tool_completed = events
        .iter()
        .find(|event| event.event_name == "agent.tool.completed")
        .expect("tool completion event should be published");
    let observation = events
        .iter()
        .find(|event| event.event_name == "agent.observation.created")
        .expect("observation event should be published");

    let payload =
        |key: &str, event: &tron_backend::shared::event_bus::BusEvent| -> Option<String> {
            event
                .payload_fields
                .iter()
                .find(|attr| attr.key == key)
                .map(|attr| attr.value.clone())
        };

    assert_eq!(
        payload("tool_call_id", tool_started).as_deref(),
        Some("RUN-TOOL-META-tool-1")
    );
    assert_eq!(payload("tool_name", tool_started).as_deref(), Some("echo"));
    assert_eq!(
        payload("tool_args_json", tool_started).as_deref(),
        Some(expected_args.as_str())
    );
    assert_eq!(
        payload("tool_call_id", tool_completed).as_deref(),
        Some("RUN-TOOL-META-tool-1")
    );
    assert_eq!(
        payload("tool_result_json", tool_completed).as_deref(),
        Some(expected_result.as_str())
    );
    assert_eq!(
        payload("tool_call_id", observation).as_deref(),
        Some("RUN-TOOL-META-tool-1")
    );
}

#[tokio::test]
async fn react_agent_truncates_oversize_tool_event_payloads_without_failing_run() {
    let oversized_output = "x".repeat(73_854);
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let oversized_output = oversized_output.clone();
            move |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: oversized_output.clone(),
                })
            }
        })))
        .expect("tool should register");
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-oversize-tool",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(250)),
    )
    .register_sink(sink.clone())
    .build();
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"command": "large"})),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("RUN-TOOL-OVERSIZE"))
        .await
        .expect("oversize tool event payload should not fail the run");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations[0].output, oversized_output);
    assert!(sink.wait_for_len(11, Duration::from_secs(2)));
    let events = sink.events();
    let tool_completed = events
        .iter()
        .find(|event| event.event_name == "agent.tool.completed")
        .expect("tool completion event should be published");
    let truncated_result = tool_completed
        .payload_fields
        .iter()
        .find(|attr| attr.key == "tool_result_json")
        .map(|attr| attr.value.clone())
        .expect("tool result payload should be present");

    assert!(truncated_result.len() <= 4096);
    assert!(truncated_result.contains("[truncated: original_bytes=73854]"));
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
            domain_code: None,
            description: "Echo".into(),
            input_schema: json!({"type": "object"}),
            skill_name: None,
            required_variable_keys: vec![],
        }],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&config, &request);
    assert_eq!(chat["model"], json!("gpt-test"));
    assert_eq!(chat["tools"][0]["function"]["name"], json!("echo"));

    let responses = responses_request(&config, &request);
    assert_eq!(responses["model"], json!("gpt-test"));
    assert_eq!(responses["input"], json!("hi"));

    let embeddings = embeddings_request(&config, "embed me");
    assert_eq!(embeddings["model"], json!("gpt-test"));
    assert_eq!(embeddings["input"], json!("embed me"));
}

#[tokio::test]
async fn max_step_exhaustion_fails_without_output_success() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
            max_parallel_tool_calls: 1,
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

#[tokio::test]
async fn zero_tool_call_limit_is_rejected_before_model_execution() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 0,
            max_parallel_tool_calls: 1,
        },
        ..base_context("RUN-ZERO-TOOL")
    };
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![])),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(context)
        .await
        .expect_err("zero tool-call limit should be rejected");

    assert_eq!(error.to_string(), "maxToolCalls must be greater than zero");
}

#[tokio::test]
async fn zero_parallel_tool_call_limit_is_rejected_before_model_execution() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
            max_parallel_tool_calls: 0,
        },
        ..base_context("RUN-ZERO-PARALLEL")
    };
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![])),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(context)
        .await
        .expect_err("zero parallel tool-call limit should be rejected");

    assert_eq!(error.to_string(), "maxParallelToolCalls must be greater than zero");
}
