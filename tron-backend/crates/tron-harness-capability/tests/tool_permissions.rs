use async_trait::async_trait;
use eliteforge_agent_framework::*;
use eliteforge_agent_test_support::{TestBuilder, TestStoragePlugin};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tron_harness_capability::{
    ToolPermission, ToolPermissionSetting, ToolPermissionSource, ToolRisk, TronHarness,
    TronRunScope, TronToolPolicy, TronToolPolicyLocale,
};

#[derive(Clone)]
struct Settings(Arc<Mutex<ToolPermissionSetting>>);
impl ToolPermissionSource for Settings {
    fn setting(&self, _: &ToolDescriptor) -> Result<ToolPermissionSetting, String> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn operation(&self, _: &ToolDescriptor) -> Option<bool> {
        None
    }
}
struct Endpoint(Arc<AtomicUsize>);
#[async_trait]
impl Tool for Endpoint {
    async fn call(&self, args: Value, _: &RunContext) -> Result<ToolOutput, ToolError> {
        assert_eq!(
            args,
            json!({"command":"unknown-program","riskLevel":"Safe"})
        );
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ToolOutput::text("executed"))
    }
}
struct Model {
    calls: Arc<AtomicUsize>,
    settings: Settings,
    revoke: bool,
    expect_visible: bool,
}
#[async_trait]
impl ChatModel for Model {
    async fn generate(
        &self,
        _: &str,
        request: ChatRequest,
        _: &dyn ChatStream,
    ) -> Result<ChatResponse, ModelError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            assert_eq!(
                request.tools.iter().any(|t| t.call_name == "native_test"),
                self.expect_visible
            );
            if self.revoke {
                self.settings.0.lock().unwrap().permission = ToolPermission::No;
            }
            Ok(ChatResponse::tool_calls(vec![ToolCall::new(
                "call",
                "native_test",
                json!({"command":"unknown-program","riskLevel":"Safe"}),
            )]))
        } else {
            Ok(ChatResponse::text("done"))
        }
    }
}
fn scope(request: &mut RunRequest) {
    TronRunScope {
        platform_scope: false,
        user_code: "U".into(),
        workspace_code: "W".into(),
        session_code: request.session_id.clone(),
        run_code: "R".into(),
        app_code: None,
        task_code: None,
        agent_code: None,
    }
    .attach(request)
    .unwrap();
}
async fn fixture(
    permission: ToolPermission,
    level: ToolRisk,
    revoke: bool,
) -> (Agent, Settings, Arc<AtomicUsize>) {
    let settings = Settings(Arc::new(Mutex::new(ToolPermissionSetting {
        tool_code: "native.test".into(),
        permission,
        confirm_level: level,
    })));
    let count = Arc::new(AtomicUsize::new(0));
    let skills = tempfile::tempdir().unwrap();
    let agent = TestBuilder::new()
        .capability_skills_root(skills.path())
        .plugin(TestStoragePlugin::default())
        .plugin(ChatModelPlugin::new(Model {
            calls: Arc::new(AtomicUsize::new(0)),
            settings: settings.clone(),
            revoke,
            expect_visible: permission != ToolPermission::No,
        }))
        .capability_pack(
            TronHarness::new(
                Arc::new(StaticToolProvider::new(vec![ToolBinding::new(
                    ToolDescriptor::new("native.test", "native_test", "", json!({"type":"object"})),
                    Endpoint(count.clone()),
                )])),
                [],
            )
            .with_tool_policy(TronToolPolicy(Arc::new(settings.clone())))
            .capability_pack(),
        )
        .build()
        .await
        .unwrap();
    (agent, settings, count)
}
async fn run(agent: &Agent) -> RunResult {
    let mut r = RunRequest::new("S", "execute");
    scope(&mut r);
    agent.run(r).await.unwrap()
}
#[tokio::test]
async fn native_approval_uses_original_arguments_once_and_denial_does_not_execute() {
    for approved in [false, true] {
        let (agent, _, count) = fixture(ToolPermission::Ask, ToolRisk::High, false).await;
        let first = run(&agent).await;
        assert_eq!(first.status, RunStatus::AwaitingInput);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let mut resumed = agent
            .answer("S", first.interactions[0].id, json!(approved))
            .await
            .unwrap();
        scope(&mut resumed);
        assert_eq!(
            agent.run(resumed).await.unwrap().status,
            RunStatus::Completed
        );
        assert_eq!(count.load(Ordering::SeqCst), usize::from(approved));
    }
}
#[tokio::test]
async fn no_excludes_discovery_and_denies_direct_calls() {
    let (agent, _, count) = fixture(ToolPermission::No, ToolRisk::Low, false).await;
    assert_eq!(run(&agent).await.status, RunStatus::Completed);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn current_permission_is_checked_after_catalog_admission() {
    let (agent, _, count) = fixture(ToolPermission::Run, ToolRisk::Low, true).await;
    assert_eq!(run(&agent).await.status, RunStatus::Completed);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn changing_permission_to_no_invalidates_an_already_approved_pending_call() {
    let (agent, settings, count) = fixture(ToolPermission::Ask, ToolRisk::High, false).await;
    let first = run(&agent).await;
    assert_eq!(first.status, RunStatus::AwaitingInput);
    let mut resumed = agent
        .answer("S", first.interactions[0].id, json!(true))
        .await
        .unwrap();
    scope(&mut resumed);
    settings.0.lock().unwrap().permission = ToolPermission::No;
    // Native Framework rejects the now-unavailable pending contract before execution.
    assert!(agent.run(resumed).await.is_err());
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn ask_only_requires_approval_at_or_above_threshold() {
    let (agent, _, count) = fixture(ToolPermission::Ask, ToolRisk::Critical, false).await;
    assert_eq!(run(&agent).await.status, RunStatus::Completed);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn critical_threshold_still_requires_approval_for_a_shell_wrapped_destructive_command() {
    struct WrappedModel;
    #[async_trait]
    impl ChatModel for WrappedModel {
        async fn generate(
            &self,
            _: &str,
            _: ChatRequest,
            _: &dyn ChatStream,
        ) -> Result<ChatResponse, ModelError> {
            Ok(ChatResponse::tool_calls(vec![ToolCall::new(
                "wrapped",
                "system_exec",
                json!({"command":"sh","args":["-c","echo read; rm -rf /"],"riskLevel":"Safe"}),
            )]))
        }
    }
    let settings = Settings(Arc::new(Mutex::new(ToolPermissionSetting {
        tool_code: "system.command.exec".into(),
        permission: ToolPermission::Ask,
        confirm_level: ToolRisk::Critical,
    })));
    let count = Arc::new(AtomicUsize::new(0));
    let skills = tempfile::tempdir().unwrap();
    let agent = TestBuilder::new()
        .capability_skills_root(skills.path())
        .plugin(TestStoragePlugin::default())
        .plugin(ChatModelPlugin::new(WrappedModel))
        .plugin(ToolProviderPlugin::new(
            "native.system",
            StaticToolProvider::new(vec![ToolBinding::new(
                ToolDescriptor::new(
                    "system.command.exec",
                    "system_exec",
                    "",
                    json!({"type":"object"}),
                ),
                Endpoint(count.clone()),
            )]),
        ))
        .capability_pack(
            TronHarness::new(Arc::new(StaticToolProvider::new(vec![])), [])
                .with_tool_policy(TronToolPolicy(Arc::new(settings)))
                .capability_pack(),
        )
        .build()
        .await
        .unwrap();
    let first = run(&agent).await;
    assert_eq!(first.status, RunStatus::AwaitingInput);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

struct LocalizedSource {
    permission: Arc<Mutex<ToolPermission>>,
    operation: Option<bool>,
}
impl ToolPermissionSource for LocalizedSource {
    fn setting(&self, descriptor: &ToolDescriptor) -> Result<ToolPermissionSetting, String> {
        Ok(ToolPermissionSetting {
            tool_code: descriptor.id.clone(),
            permission: *self.permission.lock().unwrap(),
            confirm_level: ToolRisk::Safe,
        })
    }
    fn operation(&self, _: &ToolDescriptor) -> Option<bool> {
        self.operation
    }
}
struct LocalizedModel {
    permission: Arc<Mutex<ToolPermission>>,
    revoke: bool,
    tool_name: String,
    args: Value,
    messages: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl ChatModel for LocalizedModel {
    async fn generate(
        &self,
        _: &str,
        request: ChatRequest,
        _: &dyn ChatStream,
    ) -> Result<ChatResponse, ModelError> {
        let results: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role == Role::Tool)
            .collect();
        if !results.is_empty() {
            self.messages
                .lock()
                .unwrap()
                .extend(results.iter().map(|m| m.content.text()));
            return Ok(ChatResponse::text("done"));
        }
        if self.revoke {
            *self.permission.lock().unwrap() = ToolPermission::No;
        }
        Ok(ChatResponse::tool_calls(vec![ToolCall::new(
            "localized-call",
            &self.tool_name,
            self.args.clone(),
        )]))
    }
}
struct NeverExecute;
#[async_trait]
impl Tool for NeverExecute {
    async fn call(&self, _: Value, _: &RunContext) -> Result<ToolOutput, ToolError> {
        panic!("pending or denied localized calls must not execute")
    }
}
async fn localized_fixture(
    id: &str,
    name: &str,
    args: Value,
    operation: Option<bool>,
    revoke: bool,
) -> (Agent, Arc<Mutex<Vec<String>>>, tempfile::TempDir) {
    let permission = Arc::new(Mutex::new(ToolPermission::Ask));
    let messages = Arc::new(Mutex::new(vec![]));
    let skills = tempfile::tempdir().unwrap();
    let agent = TestBuilder::new()
        .capability_skills_root(skills.path())
        .plugin(TestStoragePlugin::default())
        .plugin(ChatModelPlugin::new(LocalizedModel {
            permission: permission.clone(),
            revoke,
            tool_name: name.into(),
            args,
            messages: messages.clone(),
        }))
        .plugin(ToolProviderPlugin::new(
            "fixture.localized",
            StaticToolProvider::new(vec![ToolBinding::new(
                ToolDescriptor::new(id, name, "", json!({"type":"object"})),
                NeverExecute,
            )]),
        ))
        .capability_pack(
            TronHarness::new(Arc::new(StaticToolProvider::new(vec![])), [])
                .with_tool_policy(TronToolPolicy(Arc::new(LocalizedSource {
                    permission,
                    operation,
                })))
                .capability_pack(),
        )
        .build()
        .await
        .unwrap();
    (agent, messages, skills)
}
fn localized_request(session: &str, locale: Option<TronToolPolicyLocale>) -> RunRequest {
    let mut request = RunRequest::new(session, "execute");
    scope(&mut request);
    if let Some(locale) = locale {
        request.extensions.insert(locale);
    }
    request
}
#[tokio::test]
async fn approval_prompts_and_persisted_interactions_follow_each_run_locale() {
    let cases = [
        (
            "system.command.list",
            "terminal_list",
            None,
            json!({}),
            "系统命令",
            "Command risk",
        ),
        (
            "system.command.exec",
            "system_exec",
            None,
            json!({"command":"sh","args":["-c","rm -rf /"]}),
            "系统命令",
            "Command risk",
        ),
        (
            "mcp/server/tool",
            "mcp__server__tool",
            None,
            json!({}),
            "远端 MCP",
            "remote MCP",
        ),
        (
            "personai.generate_image",
            "generate_image",
            None,
            json!({}),
            "可能产生费用",
            "may incur charges",
        ),
        (
            "filesystem.read_text",
            "read_text",
            None,
            json!({"path":"/tmp/example"}),
            "本地文件",
            "Local file access",
        ),
        (
            "system.command.exec",
            "system_exec",
            None,
            json!({"command":"unknown-program"}),
            "系统命令",
            "Command risk",
        ),
        (
            "business.update",
            "update_record",
            Some(true),
            json!({}),
            "修改业务数据",
            "changes business data",
        ),
        (
            "business.query",
            "query_record",
            Some(false),
            json!({}),
            "用于查询",
            "performs queries",
        ),
        (
            "native.unknown",
            "unknown_tool",
            None,
            json!({}),
            "未知原生工具",
            "unknown native tool",
        ),
    ];
    for (id, name, operation, mut args, zh_reason, en_reason) in cases {
        // A model-supplied locale must not override the host's display language.
        args["locale"] = json!("en-US");
        let (agent, _, _skills) = localized_fixture(id, name, args, operation, false).await;
        let (zh, en) = tokio::join!(
            agent.run(localized_request("ZH", Some(TronToolPolicyLocale::ZhCn))),
            agent.run(localized_request("EN", Some(TronToolPolicyLocale::EnUs))),
        );
        for (session, result, reason, question, threshold) in [
            (
                "ZH",
                zh.unwrap(),
                zh_reason,
                "是否执行本次调用？",
                "确认阈值无风险",
            ),
            (
                "EN",
                en.unwrap(),
                en_reason,
                "Execute this call?",
                "Safe approval threshold",
            ),
        ] {
            assert_eq!(result.status, RunStatus::AwaitingInput);
            let interaction = &result.interactions[0];
            let prompt = interaction.prompt.text();
            assert!(prompt.contains(name), "{prompt}");
            assert!(prompt.contains(reason), "{prompt}");
            assert!(prompt.contains(question), "{prompt}");
            assert!(prompt.contains(threshold), "{prompt}");
            if session == "ZH" {
                assert!(
                    !prompt.contains("High")
                        && !prompt.contains("Low")
                        && !prompt.contains("Medium"),
                    "{prompt}"
                );
            } else {
                assert!(
                    !prompt
                        .chars()
                        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                    "{prompt}"
                );
            }
            let saved = agent.interaction(session, interaction.id).await.unwrap();
            assert_eq!(saved.request.prompt.text(), prompt);
        }
    }
}
#[tokio::test]
async fn missing_policy_locale_uses_backend_chinese_fallback() {
    let (agent, _, _skills) = localized_fixture(
        "native.unknown",
        "unknown_tool",
        json!({"locale":"en-US"}),
        None,
        false,
    )
    .await;
    let result = agent.run(localized_request("DEFAULT", None)).await.unwrap();
    assert_eq!(result.status, RunStatus::AwaitingInput);
    assert!(
        result.interactions[0]
            .prompt
            .text()
            .contains("风险为高，达到确认阈值无风险")
    );
}
#[tokio::test]
async fn permission_revocation_returns_localized_denial_without_executing() {
    for (locale, expected) in [
        (TronToolPolicyLocale::ZhCn, "该工具没有平台执行权限"),
        (
            TronToolPolicyLocale::EnUs,
            "This tool has no platform permission",
        ),
    ] {
        let (agent, messages, _skills) =
            localized_fixture("native.unknown", "unknown_tool", json!({}), None, true).await;
        let result = agent
            .run(localized_request("DENIED", Some(locale)))
            .await
            .unwrap();
        assert_eq!(result.status, RunStatus::Completed);
        assert!(result.interactions.is_empty());
        assert!(
            messages
                .lock()
                .unwrap()
                .iter()
                .any(|text| text.contains(expected)),
            "{:?}",
            messages.lock().unwrap()
        );
    }
}
