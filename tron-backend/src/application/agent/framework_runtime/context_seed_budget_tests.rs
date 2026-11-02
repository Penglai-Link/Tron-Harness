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
struct Capture(StdMutex<Vec<fw::ChatRequest>>, std::sync::atomic::AtomicUsize, std::sync::atomic::AtomicBool, StdMutex<std::collections::VecDeque<String>>, StdMutex<Vec<ModelRequest>>);
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
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.4.lock().unwrap().push(request.clone());
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.2.load(std::sync::atomic::Ordering::SeqCst) { return Err("summary provider unavailable".into()); }
            let summary = self.3.lock().unwrap().pop_front().unwrap_or_else(|| "kept evidence".into());
            Ok(ModelResponse::final_output(serde_json::json!({"summary":summary}).to_string()))
        })
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
    ]
    .into();
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
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_skills_are_separate_dynamic_data_with_independent_budget() {
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
        let host_rule = requests[0]
            .messages
            .iter()
            .find(|message| message.content.text() == "trusted workspace rule")
            .expect("host rule");
        assert_eq!(host_rule.instruction_origin, None);
        let catalog = requests[0]
            .messages
            .iter()
            .find(|message| message.content.text().contains("<available_skills>"))
            .expect("skill catalog");
        assert_eq!(catalog.role, fw::Role::User);
        assert_eq!(catalog.instruction_origin, None);
        let reports = reports.0.lock().unwrap();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert_eq!(
            report
                .blocks
                .iter()
                .map(|b| b.id.as_str())
                .collect::<Vec<_>>(),
            [
                fw::INSTRUCTIONS_BLOCK,
                "conversation_history_summary",
                "tool_domains",
                fw::HISTORY_BLOCK,
                "skills.catalog",
                "page_context",
                "application_context",
                "current_session_working_state",
                "knowledge_base"
            ]
        );
        let mut offset = 0;
        for block in &report.blocks {
            let messages = &requests[0].messages[offset..offset + block.final_message_count];
            if block.id == fw::HISTORY_BLOCK {
                assert_eq!(messages.last().unwrap().content.text(), "question");
            }
            offset += block.final_message_count;
        }
        assert_eq!(offset, requests[0].messages.len());
        let instruction = report
            .blocks
            .iter()
            .find(|block| block.id == fw::INSTRUCTIONS_BLOCK)
            .unwrap();
        let available = report.input_capacity - report.protocol_tokens;
        let pool = 16_000.min(available / 2);
        assert!(instruction.final_tokens > 0 && instruction.final_tokens <= pool);
        assert_eq!(instruction.allocated_tokens, instruction.final_tokens);
        assert_eq!(instruction.configured_tokens, None);
        assert_eq!(instruction.compression_trigger, fw::ContextCompressionTrigger::None);
        assert_eq!(
            instruction
                .sources
                .iter()
                .filter(|source| source.id == "skills.guidance")
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
            available
                - report
                    .blocks
                    .iter()
                    .filter(|block| block.id != fw::HISTORY_BLOCK)
                    .map(|block| block.final_tokens)
                    .sum::<usize>()
        );
        drop(reports);
        drop(requests);
        assert!(host.close().await.is_success());
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_oversized_skills_fail_closed_before_model_admission() {
    let (host, harness, client, _, reads) = fixture("x".repeat(100_000)).await;
    assert!(harness.run(context(131_072)).await.is_err());
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    assert!(client.0.lock().unwrap().is_empty());
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_page_context_and_steering_share_the_configured_contributor() {
    use crate::application::agent::{AgentPageContext, AgentSteerInput};
    let (host, harness, client, reports, _) = fixture("stable skill".into()).await;
    for current in [Some("/latest-page"), None] {
        let mut ctx = context(131_072);
        let mut older = AgentMessage::user("old question");
        crate::application::agent::page_context::record_snapshot(&mut older, Some(&AgentPageContext {
            path: Some("/stale-page".into()), ..Default::default()
        }), "en-US");
        let latest = harness.framework_steering(&ctx, vec![AgentSteerInput {
            content: "latest question".into(), attachments: vec![],
            annotations: vec![], steer_code: "steer-fixture".into(), requested_skills: vec![],
            page_context: current.map(|path| AgentPageContext { path: Some(path.into()), ..Default::default() }),
        }]).unwrap().remove(0);
        ctx.messages = vec![AgentMessage::system("trusted rule"), older, AgentMessage::assistant("old answer"), latest].into();
        let original = ctx.messages.clone();
        harness.run(ctx).await.unwrap();
        // Persisted input is source material, never rewritten by model assembly.
        assert!(serde_json::to_string(&original).unwrap().contains("/stale-page"));
    }
    {
        let requests = client.0.lock().unwrap();
        let reports = reports.0.lock().unwrap();
        for (index, request) in requests.iter().enumerate() {
            let report = &reports[index];
            let page = report.blocks.iter().find(|block| block.id == "page_context").unwrap();
            assert!(page.position > report.blocks.iter().find(|block| block.id == "skills.catalog").unwrap().position);
            let mut offset = 0;
            for block in &report.blocks {
                let entries = &request.messages[offset..offset + block.final_message_count];
                let text = serde_json::to_string(entries).unwrap();
                assert!(!text.contains("/stale-page"));
                if block.id == "page_context" {
                    assert_eq!(block.final_message_count, usize::from(index == 0));
                    assert_eq!(text.contains("/latest-page"), index == 0);
                } else {
                    assert!(!text.contains("/latest-page"));
                    assert!(!text.contains("tron_ui_page_context"));
                }
                offset += block.final_message_count;
            }
            let wire = crate::adapters::outbound::model::http_model_client::native_request(request).unwrap();
            let model = eliteforge_agent_official_plugin_model_http::AgentModelConfig::from(&context(131_072).model);
            let body = eliteforge_agent_official_plugin_model_http::protocol::adapter_for_protocol(model.protocol).unwrap().request_body(&model, &wire).unwrap();
            let text = body.to_string();
            assert!(!text.contains("tronMessagePart"));
            assert!(!text.contains("/stale-page"));
            assert_eq!(text.contains("/latest-page"), index == 0);
        }
    }
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changing_execution_scope_is_suffix_data_even_when_source_role_is_system() {
    let (host, harness, client, reports, _) = fixture("stable skill".into()).await;
    let root = tempfile::tempdir().unwrap();
    for revision in ["A", "B"] {
        let mut ctx = context(131_072);
        let directory = root.path().join(format!("scope-{revision}"));
        std::fs::create_dir(&directory).unwrap();
        ctx.execution_scope.local_directory = Some(directory.display().to_string());
        let scope =
            crate::application::agent::execution_context_message(&ctx.execution_scope).unwrap();
        Arc::make_mut(&mut ctx.messages).insert(1, scope);
        Arc::make_mut(&mut ctx.messages).insert(2, crate::domain::agent::AgentMessage::new("user", vec![crate::domain::agent::AgentMessagePart::Context {
            kind: "tool_domains".into(), text: format!("authorized-domain-{revision}"), metadata: None,
        }]));
        harness.run(ctx).await.unwrap();
    }
    let captured = client.0.lock().unwrap();
    let reports = reports.0.lock().unwrap();
    let mut prefixes = Vec::new();
    for (index, request) in captured.iter().enumerate() {
        let mut offset = 0;
        for block in &reports[index].blocks {
            let messages = &request.messages[offset..offset + block.final_message_count];
            if block.id == fw::INSTRUCTIONS_BLOCK {
                assert!(!serde_json::to_string(messages).unwrap().contains("/scope-"));
                assert!(!serde_json::to_string(messages).unwrap().contains("authorized-domain-"));
                prefixes.push(messages.to_vec());
            }
            if block.id == fw::HISTORY_BLOCK {
                assert!(!serde_json::to_string(messages).unwrap().contains("authorized-domain-"));
                assert!(!serde_json::to_string(messages).unwrap().contains("/scope-"));
            }
            if block.id == "tool_domains" {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].role, fw::Role::User);
                assert!(serde_json::to_string(messages).unwrap().contains(if index == 0 {"authorized-domain-A"} else {"authorized-domain-B"}));
                assert!(block.position < reports[index].blocks.iter().find(|b| b.id == fw::HISTORY_BLOCK).unwrap().position);
            }
            if block.id == "application_context" {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].role, fw::Role::User);
                assert!(
                    serde_json::to_string(&messages[0])
                        .unwrap()
                        .contains(if index == 0 { "/scope-A" } else { "/scope-B" })
                );
                let history_position = reports[index]
                    .blocks
                    .iter()
                    .find(|b| b.id == fw::HISTORY_BLOCK)
                    .unwrap()
                    .position;
                assert!(block.position > history_position);
            }
            offset += block.final_message_count;
        }
    }
    assert_eq!(prefixes.len(), 2);
    assert_eq!(prefixes[0], prefixes[1]);
    drop(captured);
    drop(reports);
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uncapped_tool_domains_stay_complete_before_history() {
    let (host, harness, client, reports, _) = fixture("stable skill".into()).await;
    let mut ctx = context(131_072);
    Arc::make_mut(&mut ctx.messages).insert(1, AgentMessage::new("user", vec![
        crate::domain::agent::AgentMessagePart::Context {
            kind: "tool_domains".into(),
            text: format!("authorized-app-domain {}", "domain description ".repeat(12_000)),
            metadata: None,
        },
    ]));
    let result = harness.run(ctx).await.unwrap();
    assert_eq!(result.final_output, "done");
    {
        let captured = client.0.lock().unwrap();
        let request = &captured[0];
        let report = &reports.0.lock().unwrap()[0];
        let block = report.blocks.iter().find(|b| b.id == "tool_domains").unwrap();
        assert_eq!(block.final_tokens, block.original_tokens.unwrap());
        assert!(block.final_tokens > 16_000, "exercise the former block cap");
        assert_eq!(block.configured_tokens, None);
        assert_eq!(block.compression_trigger, fw::ContextCompressionTrigger::None);
        assert!(block.position < report.blocks.iter().find(|b| b.id == fw::HISTORY_BLOCK).unwrap().position);
        let wire = crate::adapters::outbound::model::http_model_client::native_request(request).unwrap();
        let model = eliteforge_agent_official_plugin_model_http::AgentModelConfig::from(&context(131_072).model);
        let body = eliteforge_agent_official_plugin_model_http::protocol::adapter_for_protocol(model.protocol)
            .unwrap().request_body(&model, &wire).unwrap();
        let text = body.to_string();
        assert!(body["messages"].as_array().unwrap().iter().any(|message| message["content"].as_str() == Some(format!("authorized-app-domain {}", "domain description ".repeat(12_000)).as_str())));
        assert!(text.contains("authorized-app-domain"));
        assert!(!text.contains("\"tronMessagePart\""));
        assert!(!text.contains("\"kind\":\"tool_domains\""));
    }
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn instructions_above_the_former_cap_remain_complete() {
    let (host, harness, client, reports, _) = fixture("stable skill".into()).await;
    let mut ctx = context(131_072);
    let instruction = "protected instruction ".repeat(10_000);
    Arc::make_mut(&mut ctx.messages).insert(0, AgentMessage::system(instruction.clone()));
    harness.run(ctx).await.unwrap();
    {
        let requests = client.0.lock().unwrap();
        assert!(requests[0].messages.iter().any(|message| message.content.text() == instruction));
        let reports = reports.0.lock().unwrap();
        let block = reports[0].blocks.iter().find(|block| block.id == fw::INSTRUCTIONS_BLOCK).unwrap();
        assert!(block.final_tokens > 16_000);
        assert_eq!(block.final_tokens, block.original_tokens.unwrap());
        assert_eq!(block.configured_tokens, None);
        assert_eq!(block.compression_trigger, fw::ContextCompressionTrigger::None);
    }
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_command_reuses_all_contributors_and_saved_data_without_a_reply_call() {
    let (host, harness, client, reports, _) = fixture("stable skill".into()).await;
    let session = fw::RunId::new_v4().to_string();
    let data = AgentMessage::new("user", vec![crate::domain::agent::AgentMessagePart::Context {
        kind: "application_context".into(), text: "application fact ".repeat(600), metadata: None,
    }]);
    let mut first = context(131_072);
    first.conversation_code = Some(session.clone());
    first.messages = vec![AgentMessage::system("fixed instruction"), data.clone(),
        AgentMessage::user("old evidence ".repeat(800)), AgentMessage::assistant("prior answer"),
        AgentMessage::user("recent raw question"), AgentMessage::assistant("recent raw answer"), AgentMessage::user("/compact")].into();
    let result = harness.run(first).await.unwrap();
    assert_eq!(result.final_output, "");
    assert!(client.0.lock().unwrap().is_empty(), "maintenance must not invoke the reply model");
    let auxiliary_calls = client.1.load(std::sync::atomic::Ordering::SeqCst);
    assert!(auxiliary_calls > 0);
    let stored = host.storage_framework().await.unwrap().history().unwrap().read_session(
        &session, fw::HistoryQuery { limit:20, cursor:None },
    ).await.unwrap();
    assert!(stored.records.steps.iter().any(|step| step.name == fw::CONTEXT_COMPACTION_STEP && step.status == fw::HistoryStatus::Completed && step.model.is_none()), "maintenance status must survive outside the transient AG-UI stream");
    let first_report = reports.0.lock().unwrap()[0].clone();
    let app = first_report.blocks.iter().find(|b| b.id == "application_context").unwrap();
    assert!(app.final_tokens < app.original_tokens.unwrap());
    assert_eq!(app.compression_trigger, fw::ContextCompressionTrigger::Global);
    for id in [fw::INSTRUCTIONS_BLOCK, "tool_domains", "skills.catalog"] {
        let block = first_report.blocks.iter().find(|b| b.id == id).unwrap();
        assert_eq!(block.compression_trigger, fw::ContextCompressionTrigger::None);
    }
    let mut next = context(131_072);
    next.conversation_code = Some(session);
    next.messages = vec![AgentMessage::system("fixed instruction"), data, AgentMessage::user("continue")].into();
    harness.run(next).await.unwrap();
    assert_eq!(client.0.lock().unwrap().len(), 1);
    assert_eq!(client.1.load(std::sync::atomic::Ordering::SeqCst), auxiliary_calls, "a fitting saved projection must not invoke the compressor again");
    let report = reports.0.lock().unwrap()[1].clone();
    let reused = report.blocks.iter().find(|b| b.id == "application_context").unwrap();
    assert_eq!(reused.original_tokens, Some(app.final_tokens));
    assert_eq!(reused.final_tokens, app.final_tokens);
    let history = report.blocks.iter().find(|b| b.id == fw::HISTORY_BLOCK).unwrap();
    assert_eq!(history.final_tokens, history.original_tokens.unwrap(), "threshold accounting must use the persisted projection, not the uncompressed transcript");
    {
        let requests = client.0.lock().unwrap();
        let text = requests[0].messages.iter().map(|m| m.content.text()).collect::<Vec<_>>().join(" ");
        assert!(text.contains("kept evidence"));
        assert!(text.contains("continue"));
    }
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_failure_keeps_a_terminal_history_step_without_an_assistant_reply() {
    let (host, harness, client, _, _) = fixture("stable skill".into()).await;
    client.2.store(true, std::sync::atomic::Ordering::SeqCst);
    let session = fw::RunId::new_v4().to_string();
    let mut ctx = context(131_072);
    ctx.conversation_code = Some(session.clone());
    ctx.messages = vec![AgentMessage::system("instruction"), AgentMessage::user("old fact ".repeat(800)), AgentMessage::assistant("recorded"),
        AgentMessage::user("recent raw question"), AgentMessage::assistant("recent raw answer"), AgentMessage::user("/compact")].into();
    assert!(harness.run(ctx).await.is_err());
    assert!(client.0.lock().unwrap().is_empty());
    let history = host.storage_framework().await.unwrap().history().unwrap().read_session(&session, fw::HistoryQuery {limit:20,cursor:None}).await.unwrap();
    assert!(history.records.steps.iter().any(|step| step.name == fw::CONTEXT_COMPACTION_STEP && step.status == fw::HistoryStatus::Failed));
    assert!(host.close().await.is_success());
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_chinese_summary_retries_overlong_draft_and_persists_only_valid_projection() {
    let (host, harness, client, reports, _) = fixture("stable skill".into()).await;
    client.3.lock().unwrap().extend(["测".repeat(6000), "项目代号青松；下一步补回归测试。".into()]);
    let session = fw::RunId::new_v4().to_string();
    let mut ctx = context(131_072);
    ctx.locale = "zh-CN".into();
    ctx.conversation_code = Some(session.clone());
    ctx.messages = vec![AgentMessage::system("保留准确标识符"),
        AgentMessage::user("项目代号青松，下一步补回归测试。".repeat(300)),
        AgentMessage::assistant("已记录"), AgentMessage::user("近期原文问题"), AgentMessage::assistant("近期原文回答"), AgentMessage::user("/compact")].into();
    assert_eq!(harness.run(ctx).await.unwrap().final_output, "");
    assert!(client.0.lock().unwrap().is_empty(), "compact must not invoke the reply model");
    assert_eq!(client.1.load(Ordering::SeqCst), 2);
    {
        let requests = client.4.lock().unwrap();
        let parameters = |request: &ModelRequest| serde_json::from_str::<serde_json::Value>(&request.messages.last().unwrap().plain_text_content()).unwrap();
        let first = parameters(&requests[0]);
        let retry = parameters(&requests[1]);
        assert_eq!(first["summary_max_utf8_bytes"], 6000 * 16);
        assert!(first["summary_target_characters"].as_u64().unwrap() < first["summary_max_tokens"].as_u64().unwrap());
        assert_eq!(retry["overlong_draft"]["actual_utf8_bytes"], 18000);
        assert_eq!(first["history"], retry["history"]);
    }
    let records = host.storage_framework().await.unwrap().history().unwrap()
        .read_session(&session, fw::HistoryQuery { limit:20, cursor:None }).await.unwrap();
    assert!(records.records.steps.iter().any(|step| step.name == fw::CONTEXT_COMPACTION_STEP && step.status == fw::HistoryStatus::Completed));
    let report = reports.0.lock().unwrap()[0].clone();
    let history = report.blocks.iter().find(|block| block.id == fw::HISTORY_BLOCK).unwrap();
    let summary = report.blocks.iter().find(|block| block.id == "conversation_history_summary").unwrap();
    assert_eq!(summary.final_message_count, 1);
    assert!(summary.final_tokens <= summary.configured_tokens.unwrap());
    assert_eq!(history.final_message_count, 3, "recent completed turn and /compact remain verbatim");
    assert!(history.final_tokens < history.original_tokens.unwrap());
    let mut next = context(131_072);
    next.conversation_code = Some(session);
    next.messages = vec![AgentMessage::user("继续")].into();
    harness.run(next).await.unwrap();
    assert_eq!(client.1.load(Ordering::SeqCst), 2, "valid summary should be reused");
    let requests = client.0.lock().unwrap();
    let text = requests[0].messages.iter().map(|message| message.content.text()).collect::<Vec<_>>().join(" ");
    assert!(text.contains("青松"));
    assert!(!text.contains(&"测".repeat(100)), "overlong draft must never become persisted history");
    drop(requests);
    assert!(host.close().await.is_success());
}
