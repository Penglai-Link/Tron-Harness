use super::*;
use eliteforge_agent_test_support::TestBuilder;
#[derive(Clone)]
struct InteractionModel {
    calls: Arc<AtomicUsize>,
}
#[async_trait::async_trait]
impl fw::ChatModel for InteractionModel {
    async fn generate(
        &self,
        _: &str,
        _: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        let mut response = fw::ChatResponse::text("done");
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            response.tool_calls = vec![fw::ToolCall::new(
                "original-call",
                "collect",
                json!({"value":17}),
            )];
        }
        Ok(response)
    }
}
struct NativeInteractionTool {
    input: bool,
    calls: Arc<AtomicUsize>,
}
#[async_trait::async_trait]
impl fw::Tool for NativeInteractionTool {
    async fn call(
        &self,
        arguments: serde_json::Value,
        _: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        assert_eq!(arguments, json!({"value":17}));
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.input {
            let request = fw::InteractionRequest::input(
                "Choose a label",
                json!({"type":"string","minLength":1}),
            )
            .unwrap();
            Ok(fw::ToolOutput::text("input needed").request_input(request))
        } else {
            Ok(fw::ToolOutput::text("executed"))
        }
    }
}
struct RequireNativeApproval;
#[async_trait::async_trait]
impl fw::ToolPolicy for RequireNativeApproval {
    async fn evaluate(
        &self,
        _: &fw::ToolInvocationContext<'_>,
    ) -> Result<fw::ToolPolicyDecision, fw::AgentError> {
        Ok(fw::ToolPolicyDecision::RequireApproval(
            "Approve original arguments?".into(),
        ))
    }
}
async fn interaction_agent(
    path: &std::path::Path,
    input: bool,
    model_calls: Arc<AtomicUsize>,
    tool_calls: Arc<AtomicUsize>,
) -> Agent {
    prepare_fixture_artifacts(path);
    let mut descriptor = fw::ModelDescriptor::new("pending");
    descriptor.supports_tools = Some(true);
    let mut builder=TestBuilder::new().default_models(false)
        .plugin(eliteforge_agent_framework::ModelRuntimePlugin::new(eliteforge_agent_framework::ModelRuntime::new(eliteforge_agent_framework::ModelRuntimeUpdate::new("",0,vec![])).unwrap()))
        .plugin(fw::ModelConnectionPlugin::new(fw::ModelConnection::new("fixture",Arc::new(InteractionModel{calls:model_calls}),vec![descriptor])))
        .plugin(fw::ToolProviderPlugin::new("fixture.interaction",fw::StaticToolProvider::new(vec![fw::ToolBinding::new(
            fw::ToolDescriptor::new("collect","collect","native interaction",json!({"type":"object","required":["value"],"properties":{"value":{"type":"integer"}},"additionalProperties":false})),
            NativeInteractionTool{input,calls:tool_calls})])))
        .plugin(eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin::new(path,path.with_extension("artifacts")));
    if !input {
        builder = builder.plugin(fw::ToolPolicyPlugin::new(
            "fixture.approval",
            RequireNativeApproval,
        ));
    }
    builder.build().await.unwrap()
}
#[tokio::test]
async fn generic_native_input_survives_restart_scope_schema_checks_and_resumes_without_tool_replay()
{
    let original = FrameworkHost::temporary(None).unwrap();
    let model_calls = Arc::new(AtomicUsize::new(0));
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let agent = interaction_agent(
        &original.0.path,
        true,
        model_calls.clone(),
        tool_calls.clone(),
    )
    .await;
    assert!(original.0.framework.set(agent.framework()).is_ok());
    assert!(original.0.agent.set(agent.clone()).is_ok());
    let result = agent.run(native_request("CONV-1")).await.unwrap();
    assert_eq!(result.status, fw::RunStatus::AwaitingInput);
    let id = result.interactions[0].id;
    assert!(original.close().await.is_success());
    let restored = FrameworkHost::persistent(original.0.path.clone(), None);
    let agent = interaction_agent(
        &original.0.path,
        true,
        model_calls.clone(),
        tool_calls.clone(),
    )
    .await;
    assert!(restored.0.framework.set(agent.framework()).is_ok());
    assert!(restored.0.agent.set(agent.clone()).is_ok());
    assert_eq!(
        restored
            .pending_native_interactions("CONV-1")
            .await
            .unwrap()
            .len(),
        1
    );
    let entry = crate::application::chat::ChatResumeEntry {
        interrupt_id: id.to_string(),
        status: "resolved".into(),
        payload: json!("keep the label"),
    };
    assert!(
        restored
            .validate_native_resume("OTHER", std::slice::from_ref(&entry))
            .await
            .is_err()
    );
    let invalid = crate::application::chat::ChatResumeEntry {
        payload: json!(17),
        ..entry.clone()
    };
    assert!(
        restored
            .validate_native_resume("CONV-1", &[invalid])
            .await
            .is_err()
    );
    assert_eq!(model_calls.load(Ordering::SeqCst), 1);
    restored
        .validate_native_resume("CONV-1", std::slice::from_ref(&entry))
        .await
        .unwrap();
    let mut resumed = native_request("CONV-1");
    host_interactions::apply_answers(&agent, "CONV-1", &[entry], &mut resumed)
        .await
        .unwrap();
    let result = agent.run(resumed).await.unwrap();
    assert_eq!(result.status, fw::RunStatus::Completed);
    assert_eq!(tool_calls.load(Ordering::SeqCst), 1);
    assert_eq!(model_calls.load(Ordering::SeqCst), 2);
    assert!(
        restored
            .pending_native_interactions("CONV-1")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        agent
            .interaction("CONV-1", id)
            .await
            .unwrap()
            .response
            .unwrap()
            .value,
        json!("keep the label")
    );
    assert!(restored.close().await.is_success());
}
#[tokio::test]
async fn native_approval_boolean_resume_preserves_original_arguments_and_denial_never_executes_tool()
 {
    for approved in [true, false] {
        let host = FrameworkHost::temporary(None).unwrap();
        let model_calls = Arc::new(AtomicUsize::new(0));
        let tool_calls = Arc::new(AtomicUsize::new(0));
        let agent =
            interaction_agent(&host.0.path, false, model_calls.clone(), tool_calls.clone()).await;
        assert!(host.0.framework.set(agent.framework()).is_ok());
        assert!(host.0.agent.set(agent.clone()).is_ok());
        let first = agent.run(native_request("CONV-1")).await.unwrap();
        assert_eq!(first.status, fw::RunStatus::AwaitingInput);
        assert!(matches!(
            first.interactions[0].kind,
            fw::InteractionKind::Approval { .. }
        ));
        assert_eq!(tool_calls.load(Ordering::SeqCst), 0);
        let entry = crate::application::chat::ChatResumeEntry {
            interrupt_id: first.interactions[0].id.to_string(),
            status: "resolved".into(),
            payload: json!(approved),
        };
        host.validate_native_resume("CONV-1", std::slice::from_ref(&entry))
            .await
            .unwrap();
        let mut resumed = native_request("CONV-1");
        host_interactions::apply_answers(&agent, "CONV-1", &[entry], &mut resumed)
            .await
            .unwrap();
        let result = agent.run(resumed).await.unwrap();
        assert_eq!(result.status, fw::RunStatus::Completed);
        assert_eq!(tool_calls.load(Ordering::SeqCst), usize::from(approved));
        assert!(host.close().await.is_success());
    }
}

#[tokio::test]
async fn saved_native_answer_survives_restart_and_retries_idempotently_without_tool_replay() {
    let host = FrameworkHost::temporary(None).unwrap();
    let model_calls = Arc::new(AtomicUsize::new(0));
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let agent =
        interaction_agent(&host.0.path, true, model_calls.clone(), tool_calls.clone()).await;
    assert!(host.0.framework.set(agent.framework()).is_ok());
    assert!(host.0.agent.set(agent.clone()).is_ok());
    let paused = agent.run(native_request("CONV-1")).await.unwrap();
    let id = paused.interactions[0].id;
    let entry = crate::application::chat::ChatResumeEntry {
        interrupt_id: id.to_string(),
        status: "resolved".into(),
        payload: json!("saved answer"),
    };
    host.validate_native_resume("CONV-1", std::slice::from_ref(&entry))
        .await
        .unwrap();
    let mut request = native_request("CONV-1");
    host_interactions::apply_answers(&agent, "CONV-1", std::slice::from_ref(&entry), &mut request)
        .await
        .unwrap();
    assert!(
        agent
            .pending_interactions("CONV-1")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(model_calls.load(Ordering::SeqCst), 1);
    let mut saved = agent.interaction("CONV-1", id).await.unwrap();
    saved.request.expires_at_unix_ms = Some(0);
    assert!(host_interactions::validate_record_response(&saved, entry.payload.clone()).is_ok());
    assert!(
        host_interactions::validate_record_response(&saved, json!("different answer")).is_err()
    );
    saved.response = None;
    assert!(host_interactions::validate_record_response(&saved, entry.payload.clone()).is_err());
    assert!(host.close().await.is_success());
    let restored = FrameworkHost::persistent(host.0.path.clone(), None);
    let agent =
        interaction_agent(&host.0.path, true, model_calls.clone(), tool_calls.clone()).await;
    assert!(restored.0.framework.set(agent.framework()).is_ok());
    assert!(restored.0.agent.set(agent.clone()).is_ok());
    restored
        .validate_native_resume("CONV-1", std::slice::from_ref(&entry))
        .await
        .unwrap();
    let changed = crate::application::chat::ChatResumeEntry {
        payload: json!("changed"),
        ..entry.clone()
    };
    assert!(
        restored
            .validate_native_resume("CONV-1", &[changed])
            .await
            .is_err()
    );
    let mut request = native_request("CONV-1");
    host_interactions::apply_answers(&agent, "CONV-1", std::slice::from_ref(&entry), &mut request)
        .await
        .unwrap();
    let done = agent.run(request).await.unwrap();
    assert_eq!(done.status, fw::RunStatus::Completed);
    assert_eq!(model_calls.load(Ordering::SeqCst), 2);
    assert_eq!(tool_calls.load(Ordering::SeqCst), 1);
    assert!(
        restored
            .validate_native_resume("CONV-1", std::slice::from_ref(&entry))
            .await
            .is_err()
    );
    assert_eq!(model_calls.load(Ordering::SeqCst), 2);
    assert!(restored.close().await.is_success());
}
