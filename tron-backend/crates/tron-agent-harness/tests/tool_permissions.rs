use async_trait::async_trait;
use eliteforge_agent_framework::*;
use eliteforge_agent_test_support::{TestBuilder, TestStoragePlugin};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tron_agent_harness::{
    ToolPermission, ToolPermissionSetting, ToolPermissionSource, ToolRisk, TronHarness,
    TronRunScope, TronToolPolicy,
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
    let agent = TestBuilder::new()
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
    let agent = TestBuilder::new()
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
