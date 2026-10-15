//! Genuine SkillsEndpoint + product prefix through the native FrameworkHost.
use super::*;
use crate::domain::agent::ToolRegistry;
use crate::{
    application::agent::{ModelClient, ModelStreamSink, ReactAgentBuilder},
    domain::agent::{
        AgentMessage, AgentModelConfig, AgentRunContext, EffectiveTool, ModelRequest, ModelResponse,
    },
    shared::event_bus::EventBusHandle,
};
use async_trait::async_trait;
use eliteforge_agent_official_plugin_skills::SkillsPlugin;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Mutex as StdMutex,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Catalog {
    reads: Arc<AtomicUsize>,
    description: String,
}
#[async_trait]
impl fw::SkillProvider for Catalog {
    async fn catalog(&self, _: &fw::RunContext) -> Result<fw::SkillCatalog, fw::AgentError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(fw::SkillCatalog {
            skills: vec![fw::SkillSummary {
                name: "review".into(),
                description: self.description.clone(),
                source: "fixture".into(),
                location: "fixture/review/SKILL.md".into(),
                license: None,
                compatibility: None,
                metadata: Default::default(),
                allowed_tools: None,
            }],
            diagnostics: vec![],
        })
    }
    async fn load(&self, _: &fw::RunContext, _: &str) -> Result<fw::SkillDetail, fw::AgentError> {
        panic!("catalog assembly must not load skill instructions")
    }
    async fn read_resource(
        &self,
        _: &fw::RunContext,
        _: &str,
        _: &str,
    ) -> Result<fw::SkillResource, fw::AgentError> {
        panic!("catalog assembly must not read resources")
    }
}
#[derive(Clone, Default)]
struct Reports(Arc<StdMutex<Vec<fw::ContextAssemblyReport>>>);
#[async_trait]
impl fw::EventSink for Reports {
    async fn on_event(&self, _: &fw::RunContext, event: &fw::AgentEvent) {
        if let fw::AgentEvent::ContextAssembled(report) = event {
            self.0.lock().unwrap().push(report.clone());
        }
    }
}
#[derive(Default)]
struct Capture(StdMutex<Vec<fw::ChatRequest>>);
impl ModelClient for Capture {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate_once(config, request)
    }
    fn generate_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Ok(ModelResponse::final_output("done")) })
    }
    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.0.lock().unwrap().push(request.clone());
            Ok(ModelResponse::final_output("done"))
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
fn context(window: u32) -> AgentRunContext {
    let mut context: AgentRunContext = serde_json::from_value(serde_json::json!({
        "conversationCode":null, "runCode":fw::RunId::new_v4().to_string(), "workspaceCode":"test", "userCode":"user", "locale":"en-US", "agentKind":"REACT",
        "model":{"modelCode":"MODEL-1", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://127.0.0.1:1/v1", "apiKey":"fixture", "modelName":"test", "maxTokens":128, "contextWindowTokens":window},
        "variables":[], "skills":[], "tools":[], "executionScope":{}, "limits":{"maxSteps":2, "maxToolCalls":2, "maxParallelToolCalls":1}, "messages":[], "observations":[], "startedAt":"fixture"
    })).unwrap();
    // Authority is explicit even though this fixture never dispatches a tool.
    context.tools = ["skill_load", "skill_read_resource"]
        .into_iter()
        .map(|name| EffectiveTool {
            tool_name: name.into(),
            domain_code: None,
            description: "authorized native skills".into(),
            input_schema: serde_json::json!({"type":"object"}),
            skill_name: None,
            required_variable_keys: vec![],
        })
        .collect();
    context.messages = vec![
        AgentMessage::system("trusted workspace rule"),
        AgentMessage::user("question"),
    ].into();
    context
}
async fn fixture(
    description: String,
) -> (
    FrameworkHost,
    ReactAgent,
    Arc<Capture>,
    Reports,
    Arc<AtomicUsize>,
) {
    let host = FrameworkHost::temporary(None).unwrap();
    host.prepare_artifacts().unwrap();
    let reads = Arc::new(AtomicUsize::new(0));
    let reports = Reports::default();
    let (mut packs, extensions) = host.capability_state();
    packs.insert("tron.harness".into(), false);
    packs.insert("framework.local.scoped".into(), true);
    *host.0.capability_state.write().unwrap() = packs.clone();
    let framework = host
        .framework_builder(&packs, &extensions)
        .await
        .unwrap()
        .plugin(SkillsPlugin::new(Catalog {
            reads: reads.clone(),
            description,
        }))
        .plugin(fw::EventPlugin::new("seed.reports", reports.clone()))
        .build()
        .await
        .unwrap();
    *host.0.skills.write().unwrap() = std::collections::BTreeMap::from([(
        "framework.local.scoped".into(),
        vec!["review".into()],
    )]);
    let mut selection = host
        .main_selection(&framework, &packs, &extensions)
        .unwrap();
    selection.context_assembler = Some("framework.context-assembler".into());
    let native = framework.agent(selection).build().await.unwrap();
    assert!(host.0.framework.set(framework).is_ok());
    assert!(host.0.agent.set(native).is_ok());
    let client = Arc::new(Capture::default());
    host.bind_model_client(client.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    (host, harness, client, reports, reads)
}
#[tokio::test]
async fn native_skills_share_prefix_budget_once_under_ratio_and_hard_cap() {
    for window in [8_000, 131_072] {
        let (host, harness, client, reports, reads) =
            fixture("Review this application.".into()).await;
        let result = harness.run(context(window)).await.unwrap();
        assert_eq!(result.final_output, "done");
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        let requests = client.0.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let text = requests[0]
            .messages
            .iter()
            .map(|message| message.content.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(text.matches("<available_skills>").count(), 1);
        assert_eq!(text.matches("trusted workspace rule").count(), 1);
        let reports = reports.0.lock().unwrap();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        let instruction = report
            .blocks
            .iter()
            .find(|block| block.id == fw::INSTRUCTIONS_BLOCK)
            .unwrap();
        let available = report.input_capacity - report.protocol_tokens;
        let pool = 16_000.min(available / 2);
        assert!(instruction.final_tokens > 0 && instruction.final_tokens <= pool);
        assert_eq!(instruction.allocated_tokens, instruction.final_tokens);
        assert_eq!(
            instruction
                .sources
                .iter()
                .filter(|source| source.id == "skills.catalog")
                .count(),
            1
        );
        let history = report
            .blocks
            .iter()
            .find(|block| block.id == fw::HISTORY_BLOCK)
            .unwrap();
        assert_eq!(
            history.allocated_tokens,
            available - instruction.final_tokens
        );
        drop(reports);
        drop(requests);
        assert!(host.close().await.is_success());
    }
}
#[tokio::test]
async fn native_oversized_skills_fail_closed_before_model_admission() {
    let (host, harness, client, _, reads) = fixture("x".repeat(100_000)).await;
    assert!(harness.run(context(131_072)).await.is_err());
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    assert!(client.0.lock().unwrap().is_empty());
    assert!(host.close().await.is_success());
}
