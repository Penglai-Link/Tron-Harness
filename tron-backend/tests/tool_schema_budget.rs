use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::model::{chat_completions_request, responses_request},
    application::{
        agent::{
            BusinessToolCatalog, ModelClient, ReactAgentBuilder, ToolSchemaCache,
            discovery_expand_tool_definition, tool_record_to_effective_tool,
        },
        short_term_memory::{
            ContextBlockBudgetConfig, ShortTermMemoryConfig, ShortTermMemoryService,
        },
    },
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, AgentToolExposureState, EffectiveTool,
        ExpandedDomainPersistenceMode, InProcessTool, ModelRequest, ModelResponse, Tool, ToolCall,
        ToolContextDelta, ToolError, ToolExecutionResult, ToolExposureMode, ToolObservation,
        ToolRegistry, ToolRequest,
    },
    shared::{
        event_bus::{EventBusConfig, EventBusRuntimeBuilder},
        id::FixedSnowflakeIdProvider,
    },
};

fn tool(name: &str, domain: &str, description_len: usize) -> EffectiveTool {
    EffectiveTool {
        tool_name: name.into(),
        domain_code: Some(domain.into()),
        description: "x".repeat(description_len),
        input_schema: json!({"type":"object","properties":{"query":{"type":"string"}},"additionalProperties":false}),
        skill_name: Some("internal-skill".into()),
        required_variable_keys: vec!["INTERNAL_VARIABLE".into()],
    }
}

fn context() -> AgentRunContext {
    AgentRunContext {
        collaboration_owner: None,
        agent_code: None,
        conversation_code: None,
        run_code: eliteforge_agent_core::RunId::new_v4().to_string(),
        workspace_code: "TRON_CORE".into(),
        user_code: None,
        locale: "en-US".into(),
        request_id: None,
        stream_key: None,
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: AgentModelConfig {
            model_code: "MODEL-SCHEMA".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1".into(),
            api_path: None,
            api_key: "test".into(),
            model_name: "test".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: Some(128),
            context_window_tokens: Some(32_000),
            top_p: None,
            max_retries: Some(0),
        },
        variables: Vec::new(),
        skills: Vec::new(),
        tools: vec![tool("echo", "base", 20)],
        effective_tools: Vec::new(),
        tool_assignment: None,
        tool_exposure: AgentToolExposureState::default(),
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits {
            max_steps: 8,
            max_tool_calls: 8,
            max_parallel_tool_calls: 2,
        },
        messages: vec![AgentMessage::user("exercise tools")],
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-09-07T00:00:00Z".into(),
        auto_model_route: None,
    }
}

#[derive(Default)]
struct ScriptedClient {
    responses: Mutex<VecDeque<ModelResponse>>,
    requests: Mutex<Vec<ModelRequest>>,
}

impl ScriptedClient {
    fn new(responses: Vec<ModelResponse>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        })
    }
}

impl ModelClient for ScriptedClient {
    fn generate<'a>(
        &'a self,
        _config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.clone());
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| "script exhausted".into())
        })
    }

    // Test adapters explicitly declare one transport attempt per mock invocation.
    fn generate_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        self.generate(config, request)
    }

    fn close_observed<'a>(
        &'a self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }

    fn close<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}

fn call(name: &str, domain: &str, id: &str) -> ToolCall {
    ToolCall {
        call_id: Some(id.into()),
        tool_name: name.into(),
        input: json!({"domainCode":domain}),
        input_error: None,
    }
}

fn expand(domain: &str, id: &str) -> ModelResponse {
    ModelResponse::tool_calls_with_usage(
        vec![call("discovery_expand_tool_domain", domain, id)],
        None,
    )
}

struct ExpandDomain;

impl Tool for ExpandDomain {
    fn name(&self) -> &str {
        "discovery_expand_tool_domain"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let domain = request.input["domainCode"].as_str().unwrap();
        let tools = context
            .tool_exposure
            .hidden_tools
            .iter()
            .filter(|tool| tool.domain_code.as_deref() == Some(domain))
            .cloned()
            .collect::<Vec<_>>();
        Ok(ToolExecutionResult {
            observation: ToolObservation {
                tool_name: self.name().into(),
                output: json!({"domainCode":domain,"status":"expanded","toolCount":tools.len()})
                    .to_string(),
            },
            context_delta: ToolContextDelta {
                add_tools: tools,
                expanded_domain_codes: vec![domain.into()],
            },
        })
    }
}

fn discovery_context(
    hidden: Vec<EffectiveTool>,
    mode: ExpandedDomainPersistenceMode,
) -> AgentRunContext {
    let mut context = context();
    context.tools = vec![discovery_expand_tool_definition("en-US")];
    context.effective_tools = context.tools.iter().chain(hidden.iter()).cloned().collect();
    context.tool_exposure = AgentToolExposureState {
        tool_exposure_mode: Some(ToolExposureMode::DomainDiscovery),
        expanded_domain_persistence_mode: Some(mode),
        hidden_tools: hidden,
        ..Default::default()
    };
    context
}

fn builder(
    client: Arc<ScriptedClient>,
    config: Option<ShortTermMemoryConfig>,
) -> ReactAgentBuilder {
    let registry = Arc::new(ToolRegistry::default());
    registry.register(Arc::new(ExpandDomain)).unwrap();
    // Every authorized test descriptor has an actual registered implementation.
    for name in ["echo", "work", "left_work", "right_work"] {
        registry
            .register(Arc::new(InProcessTool::new(name, |request| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "done".into(),
                })
            })))
            .unwrap();
    }
    let events = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(100)),
    )
    .build();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    ReactAgentBuilder::new(client, registry, events)
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(
            config.map(|config| ShortTermMemoryService::with_host(host, config)),
        )
}

fn limit_config(limit: u32) -> ShortTermMemoryConfig {
    let mut config = ShortTermMemoryConfig::default();
    config.context.blocks.tool_schema = ContextBlockBudgetConfig {
        max_ratio_bps: None,
        max_tokens: Some(limit),
    };
    config
}

fn estimate(tools: &[EffectiveTool]) -> u32 {
    ToolSchemaCache::default()
        .prepare(AgentModelProtocol::ChatCompletions, tools, u32::MAX)
        .unwrap()
        .estimated_tokens()
}

#[test]
fn schema_cache_reuses_snapshots_and_invalidates_definition_and_protocol_changes() {
    let mut tools = vec![tool("search", "search", 100)];
    let mut cache = ToolSchemaCache::default();
    let first = cache
        .prepare(AgentModelProtocol::ChatCompletions, &tools, u32::MAX)
        .unwrap();
    let second = cache
        .prepare(AgentModelProtocol::ChatCompletions, &tools, u32::MAX)
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.revision(), 1);
    tools[0]
        .required_variable_keys
        .push("OTHER_INTERNAL_KEY".into());
    assert!(Arc::ptr_eq(
        &first,
        &cache
            .prepare(AgentModelProtocol::ChatCompletions, &tools, u32::MAX)
            .unwrap()
    ));
    assert!(!first.payload().to_string().contains("INTERNAL_VARIABLE"));
    assert!(!first.payload().to_string().contains("internal-skill"));
    assert_eq!(
        first.estimated_tokens() as usize,
        first.payload().to_string().len().div_ceil(4)
    );
    tools[0].input_schema["properties"]["query"]["description"] = json!("new definition");
    let changed = cache
        .prepare(AgentModelProtocol::ChatCompletions, &tools, u32::MAX)
        .unwrap();
    assert_eq!(changed.revision(), 2);
    assert!(!Arc::ptr_eq(&first, &changed));
    let responses = cache
        .prepare(AgentModelProtocol::Responses, &tools, u32::MAX)
        .unwrap();
    assert_eq!(responses.revision(), 3);
    assert_eq!(responses.payload()[0]["name"], "search");
    assert!(responses.payload()[0].get("function").is_none());
    tools[0].description = "localized description".into();
    assert_eq!(
        cache
            .prepare(AgentModelProtocol::Responses, &tools, u32::MAX)
            .unwrap()
            .revision(),
        4
    );
}

#[test]
fn schema_cache_rechecks_limits_without_committing_rejected_candidates() {
    let tools = vec![tool("search", "search", 100)];
    let mut cache = ToolSchemaCache::default();
    let first = cache
        .prepare(AgentModelProtocol::ChatCompletions, &tools, u32::MAX)
        .unwrap();
    assert!(
        cache
            .prepare(
                AgentModelProtocol::ChatCompletions,
                &tools,
                first.estimated_tokens() - 1
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&first, &cache.current().unwrap()));
    let mut expanded = tools.clone();
    expanded.push(tool("large", "search", 1000));
    assert!(
        cache
            .prepare(
                AgentModelProtocol::ChatCompletions,
                &expanded,
                first.estimated_tokens()
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&first, &cache.current().unwrap()));
    assert!(
        cache
            .prepare(
                AgentModelProtocol::ChatCompletions,
                &tools,
                first.estimated_tokens()
            )
            .is_ok()
    );
    assert_eq!(
        cache
            .prepare(AgentModelProtocol::ChatCompletions, &[], 0)
            .unwrap()
            .estimated_tokens(),
        0
    );
}

#[test]
fn adapters_send_the_same_payload_that_was_measured_and_ignore_stale_snapshots() {
    for protocol in [
        AgentModelProtocol::ChatCompletions,
        AgentModelProtocol::Responses,
    ] {
        let mut context = context();
        context.model.protocol = protocol;
        let schema = ToolSchemaCache::default()
            .prepare(protocol, &context.tools, u32::MAX)
            .unwrap();
        let mut request = ModelRequest {
            messages: context.messages,
            tools: context.tools,
            prepared_tool_schema: Some(schema.clone()),
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let render = if protocol == AgentModelProtocol::ChatCompletions {
            chat_completions_request
        } else {
            responses_request
        };
        assert_eq!(&render(&context.model, &request)["tools"], schema.payload());
        assert!(
            serde_json::to_value(&request)
                .unwrap()
                .get("preparedToolSchema")
                .is_none()
        );
        let restored: ModelRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert_eq!(
            request, restored,
            "cache metadata must not affect request equality"
        );
        request.tools[0].description = "changed without the snapshot".into();
        assert_ne!(&render(&context.model, &request)["tools"], schema.payload());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_reuses_schema_within_a_run_but_not_across_runs() {
    let client = ScriptedClient::new(vec![
        ModelResponse::tool_calls_with_usage(vec![call("echo", "", "echo-1")], None),
        ModelResponse::final_output("first done"),
        ModelResponse::final_output("second done"),
    ]);
    let agent = builder(client.clone(), None).build().unwrap();
    agent.run(context()).await.unwrap();
    let mut next = context();
    next.run_code = eliteforge_agent_core::RunId::new_v4().to_string();
    agent.run(next).await.unwrap();
    let requests = client.requests.lock().unwrap();
    let snapshots = requests
        .iter()
        .map(|request| request.prepared_tool_schema.as_ref().unwrap())
        .collect::<Vec<_>>();
    assert!(Arc::ptr_eq(snapshots[0], snapshots[1]));
    assert!(!Arc::ptr_eq(snapshots[1], snapshots[2]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn initial_schema_overflow_never_calls_model_even_without_a_session() {
    for service in [false, true] {
        let client = ScriptedClient::new(vec![ModelResponse::final_output("must not execute")]);
        let agent = builder(client.clone(), service.then(|| limit_config(100)))
            .build()
            .unwrap();
        let mut context = context();
        context.tools[0].description = "large".repeat(40_000);
        let error = agent.run(context).await.unwrap_err();
        assert!(error.to_string().contains("TOOL_SCHEMA_OVER_BUDGET"));
        assert!(client.requests.lock().unwrap().is_empty());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_expansion_preserves_tools_cache_and_call_id() {
    let context = discovery_context(
        vec![tool("large", "large", 10_000)],
        ExpandedDomainPersistenceMode::StickyExpanded,
    );
    let client = ScriptedClient::new(vec![
        expand("large", "expand-1"),
        ModelResponse::final_output("cannot expand"),
    ]);
    let agent = builder(client.clone(), Some(limit_config(estimate(&context.tools))))
        .build()
        .unwrap();
    let result = agent.run(context).await.unwrap();
    assert!(
        result.observations[0]
            .output
            .contains("TOOL_SCHEMA_OVER_BUDGET")
    );
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests[0].tools, requests[1].tools);
    assert!(Arc::ptr_eq(
        requests[0].prepared_tool_schema.as_ref().unwrap(),
        requests[1].prepared_tool_schema.as_ref().unwrap()
    ));
    assert!(
        requests[1]
            .messages
            .iter()
            .any(|message| message.first_tool_call_id() == Some("expand-1")
                && message.text_content().contains("TOOL_SCHEMA_OVER_BUDGET"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_within_its_own_limit_still_counts_toward_total_input_budget() {
    for service in [false, true] {
        let mut context = context();
        context.model.context_window_tokens = Some(1000);
        context.tools[0].description = "x".repeat(300);
        context.messages = vec![AgentMessage::user("x".repeat(700))];
        let input_budget =
            context.model.context_window_tokens.unwrap() - context.model.max_tokens.unwrap();
        let message_tokens =
            eliteforge_agent_memory::estimate_serialized_tokens(&context.messages[0]);
        let schema_tokens = estimate(&context.tools);
        assert!(message_tokens < input_budget);
        assert!(schema_tokens < input_budget);
        assert!(message_tokens + schema_tokens > input_budget);
        eprintln!(
            "service={service}, message={message_tokens}, schema={schema_tokens}, combined={}, input_budget={input_budget}",
            message_tokens + schema_tokens
        );
        // Each component must be admitted separately; only their combined
        // model-visible input should exceed the window minus output reserve.
        for tools_only in [false, true] {
            let client = ScriptedClient::new(vec![ModelResponse::final_output("within budget")]);
            let agent = builder(client.clone(), service.then(ShortTermMemoryConfig::default))
                .build()
                .unwrap();
            let mut isolated = context.clone();
            if tools_only {
                isolated.messages = vec![AgentMessage::user("small input")];
            } else {
                isolated.tools.clear();
            }
            assert_eq!(
                agent.run(isolated).await.unwrap().final_output,
                "within budget"
            );
            assert_eq!(client.requests.lock().unwrap().len(), 1);
        }
        let client = ScriptedClient::new(vec![ModelResponse::final_output("must not execute")]);
        let agent = builder(client.clone(), service.then(ShortTermMemoryConfig::default))
            .build()
            .unwrap();
        let error = agent.run(context).await.unwrap_err();
        assert!(
            error.to_string().contains("context budget exceeded"),
            "service={service}: {error:?}"
        );
        assert!(client.requests.lock().unwrap().is_empty());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parallel_expansions_check_the_merged_tool_set() {
    let hidden = vec![
        tool("left_work", "left", 600),
        tool("right_work", "right", 600),
    ];
    let context = discovery_context(
        hidden.clone(),
        ExpandedDomainPersistenceMode::StickyExpanded,
    );
    let left = [context.tools.clone(), vec![hidden[0].clone()]].concat();
    let right = [context.tools.clone(), vec![hidden[1].clone()]].concat();
    let cap = estimate(&left).max(estimate(&right));
    assert!(estimate(&[context.tools.clone(), hidden].concat()) > cap);
    let client = ScriptedClient::new(vec![
        ModelResponse::tool_calls_with_usage(
            vec![
                call("discovery_expand_tool_domain", "left", "left-1"),
                call("discovery_expand_tool_domain", "right", "right-1"),
            ],
            None,
        ),
        ModelResponse::tool_call("left_work", json!({"query":"accepted domain"})),
        ModelResponse::final_output("one domain accepted"),
    ]);
    let result = builder(client.clone(), Some(limit_config(cap)))
        .build()
        .unwrap()
        .run(context)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&result.observations[0].output).unwrap()["status"],
        "expanded"
    );
    assert!(
        result.observations[1]
            .output
            .contains("TOOL_SCHEMA_OVER_BUDGET")
    );
    assert_eq!(result.final_output, "one domain accepted");
    assert_eq!(result.observations.len(), 3);
    assert_eq!(result.observations[2].tool_name, "left_work");
    assert_eq!(result.observations[2].output, "done");
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].tools, left);
    assert_eq!(requests[2].tools, left);
    assert!(
        requests[1]
            .prepared_tool_schema
            .as_ref()
            .unwrap()
            .estimated_tokens()
            <= cap
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_persistence_and_idempotence_keep_correct_snapshots() {
    for mode in [
        ExpandedDomainPersistenceMode::StickyExpanded,
        ExpandedDomainPersistenceMode::EphemeralExpanded,
    ] {
        let context = discovery_context(vec![tool("work", "work", 20)], mode);
        let client = ScriptedClient::new(vec![
            expand("work", "expand-1"),
            expand("work", "expand-2"),
            ModelResponse::tool_calls_with_usage(vec![call("work", "", "work-1")], None),
            ModelResponse::final_output("done"),
        ]);
        builder(client.clone(), None)
            .build()
            .unwrap()
            .run(context)
            .await
            .unwrap();
        let requests = client.requests.lock().unwrap();
        assert_eq!(requests[0].tools.len(), 1);
        assert_eq!(requests[1].tools.len(), 2);
        assert!(Arc::ptr_eq(
            requests[1].prepared_tool_schema.as_ref().unwrap(),
            requests[2].prepared_tool_schema.as_ref().unwrap()
        ));
        if mode == ExpandedDomainPersistenceMode::StickyExpanded {
            assert_eq!(requests[3].tools.len(), 2);
            assert!(Arc::ptr_eq(
                requests[2].prepared_tool_schema.as_ref().unwrap(),
                requests[3].prepared_tool_schema.as_ref().unwrap()
            ));
        } else {
            assert_eq!(requests[3].tools.len(), 1);
            assert_eq!(
                requests[3]
                    .prepared_tool_schema
                    .as_ref()
                    .unwrap()
                    .revision(),
                3
            );
        }
    }
}

#[test]
fn measure_builtin_catalog_schema_for_default_budget_compatibility() {
    for locale in ["en-US", "zh-CN"] {
        let tools = BusinessToolCatalog::list_tools(None, None, locale)
            .into_iter()
            .map(tool_record_to_effective_tool)
            .collect::<Vec<_>>();
        for protocol in [
            AgentModelProtocol::ChatCompletions,
            AgentModelProtocol::Responses,
        ] {
            let snapshot = ToolSchemaCache::default()
                .prepare(protocol, &tools, u32::MAX)
                .unwrap();
            println!(
                "catalog locale={locale} protocol={protocol:?} tools={} estimated_tokens={}",
                tools.len(),
                snapshot.estimated_tokens()
            );
            assert!(snapshot.estimated_tokens() > 0);
        }
    }
}
