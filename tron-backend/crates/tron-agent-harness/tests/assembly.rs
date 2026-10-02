use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use eliteforge_agent_framework::*;
use eliteforge_agent_test_support::TestStoragePlugin;
use serde_json::json;
use tron_agent_harness::{TronHarness, TronRunScope};

struct HostContext;
#[async_trait]
impl ContextContributor for HostContext {
    fn kind(&self) -> ContextSourceKind {
        ContextSourceKind::Instruction
    }
    fn block_id(&self) -> &str {
        "tron.test.instructions"
    }
    async fn contribute(&self, input: &ContextInput<'_>) -> Result<ContextBlock, AgentError> {
        let scope = TronRunScope::from_run(input.run)?;
        Ok(ContextBlock::instructions(
            self.block_id(),
            vec![Message::system(format!(
                "Workspace: {}",
                scope.workspace_code
            ))],
        ))
    }
}

struct BusinessTool(Arc<AtomicUsize>);
#[async_trait]
impl Tool for BusinessTool {
    async fn call(&self, _: serde_json::Value, run: &RunContext) -> Result<ToolOutput, ToolError> {
        let scope = TronRunScope::from_run(run).map_err(|e| ToolError::Failed(e.to_string()))?;
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ToolOutput::text(format!(
            "{}:{}",
            scope.workspace_code, scope.user_code
        )))
    }
}

fn binding(id: &str, name: &str, calls: Arc<AtomicUsize>) -> ToolBinding {
    ToolBinding::new(
        ToolDescriptor::new(
            id,
            name,
            "Tron business tool",
            json!({"type":"object","additionalProperties":false}),
        ),
        BusinessTool(calls),
    )
}

// Exercises the actual framework assembly, scope injection and ToolInvoker.
// The application's model loop and database migration have separate acceptance.
struct ProbeLoop {
    previous: Mutex<Option<ToolBinding>>,
}
#[async_trait]
impl AgentLoop for ProbeLoop {
    async fn run(
        &self,
        _: &RunRequest,
        run: &RunContext,
        services: &AgentServices,
    ) -> Result<RunResult, AgentError> {
        let mut messages = Vec::new();
        for provider in &services.context_sources {
            messages.extend(
                provider
                    .contribute(&ContextInput {
                        stage: "test",
                        step: 1,
                        model: &ModelRef::new("test", "test"),
                        budget: ContextBudget {
                            max_tokens: 100_000,
                            max_messages: 4096,
                        },
                        current_input: &Content::default(),
                        history: &ConversationSnapshot::new(
                            ConversationKey::new(&run.session_id),
                            "test",
                            Vec::new(),
                        ),
                        estimator: &ConservativeTokenEstimator,
                        run: run,
                    })
                    .await?
                    .messages,
            );
        }
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::System);
        assert!(messages[0].content.text().contains("Workspace:"));
        let mut tools = Vec::new();
        for provider in &services.tools {
            let mut lease = provider.lease(run).await?;
            run.own_resources(std::mem::take(&mut lease.resources));
            tools.extend(lease.tools);
        }
        assert_eq!(
            tools.len(),
            1,
            "harness does not install generic tools implicitly"
        );
        assert!(!tools[0].exposed, "discovery visibility is preserved");
        assert!(
            tools[0].parallel_safe,
            "host parallel-safety policy is preserved"
        );
        let previous = self.previous.lock().unwrap().replace(tools[0].clone());
        let call = ToolCall::new("call-1", &tools[0].descriptor.call_name, json!({}));
        if let Some(previous) = previous {
            let result = services
                .tool_invoker
                .invoke(
                    "old-invocation",
                    &call,
                    &previous,
                    run,
                    run.limits().tool_timeout,
                )
                .await;
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("another run scope")
            );
        }
        let result = services
            .tool_invoker
            .invoke(
                "host-invocation",
                &call,
                &tools[0],
                run,
                run.limits().tool_timeout,
            )
            .await
            .map_err(|e| AgentError::ToolSetup(e.to_string()))?;
        Ok(RunResult {
            external_session: None,
            output: result.content.text(),
            content: result.content,
            structured: result.structured,
            interactions: vec![],
            status: RunStatus::Completed,
            usage: Usage::default(),
            steps: 1,
            tool_calls: 1,
        })
    }
}

fn request(session: &str, workspace: &str) -> RunRequest {
    let mut request = RunRequest::new(session, "inspect project");
    TronRunScope {
        user_code: "USR-1".into(),
        workspace_code: workspace.into(),
        session_code: session.into(),
        run_code: format!("RUN-{session}"),
        app_code: Some("APP-1".into()),
        task_code: None,
        agent_code: None,
    }
    .attach(&mut request)
    .unwrap();
    request
}

async fn agent(tool: ToolBinding, shared: Vec<ToolDescriptor>) -> Agent {
    AgentBuilder::new()
        .default_models(false)
        .plugin(TestStoragePlugin::default())
        .plugin(AgentLoopPlugin::new(ProbeLoop {
            previous: Mutex::new(None),
        }))
        .capability_pack(
            TronHarness::new(Arc::new(StaticToolProvider::new(vec![tool])), shared)
                .with_context_source(HostContext)
                .capability_pack(),
        )
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn preserves_host_context_and_policy_without_cross_workspace_lease_reuse() {
    let calls = Arc::new(AtomicUsize::new(0));
    let agent = agent(
        binding("goal.read", "goal_read", calls.clone())
            .hidden()
            .parallel_safe(),
        vec![],
    )
    .await;
    assert_eq!(
        agent.run(request("S-1", "WS-1")).await.unwrap().output,
        "WS-1:USR-1"
    );
    assert_eq!(
        agent.run(request("S-2", "WS-2")).await.unwrap().output,
        "WS-2:USR-1"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    agent.close().await;
}

#[tokio::test]
async fn missing_or_mismatched_scope_fails_before_host_tool_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let agent = agent(binding("goal.read", "goal_read", calls.clone()), vec![]).await;
    assert!(
        agent
            .run(RunRequest::new("S-1", "input"))
            .await
            .unwrap_err()
            .to_string()
            .contains("scope is missing")
    );
    let mut wrong = request("S-1", "WS-1");
    wrong.session_id = "S-2".into();
    assert!(
        agent
            .run(wrong)
            .await
            .unwrap_err()
            .to_string()
            .contains("does not belong")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    agent.close().await;
}

#[tokio::test]
async fn cannot_shadow_local_tool_by_name_or_shared_plugin_by_id() {
    for (id, name, shared) in [
        ("tron.fake_read", "fs_read_text", vec![]),
        (
            "shared.business",
            "renamed",
            vec![ToolDescriptor::new(
                "shared.business",
                "remote_read",
                "shared",
                json!({"type":"object"}),
            )],
        ),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let agent = agent(binding(id, name, calls.clone()), shared).await;
        assert!(
            agent
                .run(request("S-1", "WS-1"))
                .await
                .unwrap_err()
                .to_string()
                .contains("instead of redefining")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        agent.close().await;
    }
}
