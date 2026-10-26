use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use serde_json::json;
use tron_backend::{
    adapters::outbound::model::{chat_completions_request, responses_request},
    application::{
        agent::{
            BusinessToolCatalog, ModelClient, ReactAgentBuilder, ToolSchemaCache,
            tool_record_to_effective_tool,
        },
        short_term_memory::{ShortTermMemoryConfig, ShortTermMemoryService},
    },
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveTool,
        InProcessTool, ModelRequest, ModelResponse, ToolCall,
        ToolObservation, ToolRegistry,
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
        system_agent_fast_mode: false,
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
            supports_developer_role: false,
        },
        variables: Vec::new(),
        skills: Vec::new(),
        tools: vec![tool("echo", "base", 20)],
        effective_tools: Vec::new(),
        tool_assignment: None,
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits {
            max_steps: 8,
            max_tool_calls: 8,
            max_parallel_tool_calls: 2,
        },
        messages: vec![AgentMessage::user("exercise tools")].into(),
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


fn builder(
    client: Arc<ScriptedClient>,
    config: Option<ShortTermMemoryConfig>,
) -> ReactAgentBuilder {
    let registry = Arc::new(ToolRegistry::default());
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
        eliteforge_agent_core::estimate_unicode_text_tokens(&first.payload().to_string())
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
            messages: context.messages.as_ref().clone(),
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
async fn inner_tool_bloat_never_reaches_the_model_schema_budget() {
    // Gateway mode: the model-visible schema is exactly the stable pair, so
    // an oversized business-tool description cannot overflow the schema
    // budget or block the run. This is the cache-stability guarantee.
    for service in [false, true] {
        let client = ScriptedClient::new(vec![ModelResponse::final_output("done")]);
        let agent = builder(client.clone(), service.then(ShortTermMemoryConfig::default))
            .build()
            .unwrap();
        let mut context = context();
        context.tools[0].description = "large".repeat(40_000);
        let result = agent.run(context).await.unwrap();
        assert_eq!(result.final_output, "done");
        let requests = client.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let names: Vec<&str> = requests[0]
            .tools
            .iter()
            .map(|tool| tool.tool_name.as_str())
            .collect();
        assert!(names.contains(&"tool_search") && names.contains(&"tool_execute"));
        // The wrapped business tool never appears in the schema payload.
        assert!(!names.contains(&"echo"));
        if let Some(schema) = requests[0].prepared_tool_schema.as_ref() {
            assert!(schema.estimated_tokens() < 2_000);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_within_its_own_limit_still_counts_toward_total_input_budget() {
    // Gateway mode: the model-visible schema is the small, stable pair, so a
    // large business-tool description no longer threatens the total input
    // budget. The combined-input guard still fires for oversized message
    // content, and each admitted component must fit on its own.
    for service in [false, true] {
        let mut context = context();
        context.tools[0].description = "x".repeat(30_000);
        // 3 ASCII chars per token: a message alone under the input budget.
        let input_budget =
            context.model.context_window_tokens.unwrap() - context.model.max_tokens.unwrap();
        // Honor the configured history share as well as protocol/instruction
        // overhead. A message fitting the whole window may exceed its block.
        let message_len = ((input_budget / 3) as usize) * 3;
        context.messages = vec![AgentMessage::user("x".repeat(message_len))].into();

        // The message alone fits the window.
        let client = ScriptedClient::new(vec![ModelResponse::final_output("within budget")]);
        let agent = builder(client.clone(), service.then(ShortTermMemoryConfig::default))
            .build()
            .unwrap();
        let mut message_only = context.clone();
        message_only.tools.clear();
        assert_eq!(
            agent.run(message_only).await.unwrap().final_output,
            "within budget"
        );
        assert_eq!(client.requests.lock().unwrap().len(), 1);
        let schema_tokens = client.requests.lock().unwrap()[0]
            .prepared_tool_schema
            .as_ref()
            .map(|schema| schema.estimated_tokens())
            .unwrap_or_default();
        // The stable pair carries a small schema; the wrapped business tool
        // never enters it.
        assert!(schema_tokens > 0 && schema_tokens < 2_000);

        // The pair schema alone also fits.
        let client = ScriptedClient::new(vec![ModelResponse::final_output("pair only")]);
        let agent = builder(client.clone(), service.then(ShortTermMemoryConfig::default))
            .build()
            .unwrap();
        let mut pair_only = context.clone();
        pair_only.messages = vec![AgentMessage::user("small input")].into();
        assert_eq!(
            agent.run(pair_only).await.unwrap().final_output,
            "pair only"
        );

        // An oversized message alone must still be rejected before dispatch.
        let mut oversized = context.clone();
        oversized.messages =
            vec![AgentMessage::user("x".repeat((input_budget as usize + 6_000) * 3))].into();
        let client = ScriptedClient::new(vec![ModelResponse::final_output("must not execute")]);
        let agent = builder(client.clone(), service.then(ShortTermMemoryConfig::default))
            .build()
            .unwrap();
        let error = agent.run(oversized).await.unwrap_err();
        assert!(
            error.to_string().contains("context budget exceeded"),
            "service={service}: {error:?}"
        );
        assert!(client.requests.lock().unwrap().is_empty());
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
