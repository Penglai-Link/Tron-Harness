//! A provider authentication retry must pass through real Tron context preparation.
use crate::domain::agent::ToolRegistry;
use crate::{
    application::agent::{FrameworkHost, ModelClient, ReactAgentBuilder},
    domain::agent::{AgentMessage, AgentModelConfig, AgentRunContext, ModelRequest, ModelResponse},
    shared::event_bus::EventBusHandle,
};
use eliteforge_agent_core as fw;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Default)]
struct AuthenticationClient {
    calls: AtomicUsize,
    refreshes: AtomicUsize,
}
impl ModelClient for AuthenticationClient {
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
        Box::pin(async move {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                Err(
                    eliteforge_agent_official_plugin_model_http::encode_model_error(
                        &fw::ModelError::Provider(Box::new(fw::ProviderModelError {
                            authorization_attempt: None,
                            provider: "fixture".into(),
                            category: fw::ProviderErrorCategory::Authentication,
                            http_status: Some(401),
                            business_code: None,
                            error_type: None,
                            model: None,
                            message: "credential expired".into(),
                        })),
                    ),
                )
            } else {
                Ok(ModelResponse::final_output("authenticated answer"))
            }
        })
    }
    fn recover_authentication<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        error: &'a fw::ModelError,
    ) -> Pin<Box<dyn Future<Output = Result<bool, fw::ModelError>> + Send + 'a>> {
        Box::pin(async move {
            assert!(
                matches!(error, fw::ModelError::Provider(error) if error.category == fw::ProviderErrorCategory::Authentication)
            );
            self.refreshes.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        })
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
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}
#[tokio::test]
async fn sdk_authentication_recovery_reassembles_tron_context_and_dispatches_again() {
    let client = Arc::new(AuthenticationClient::default());
    let host = FrameworkHost::temporary(None).unwrap();
    host.bind_model_client(client.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context: AgentRunContext = serde_json::from_value(serde_json::json!({
        "conversationCode":null, "runCode":fw::RunId::new_v4().to_string(), "workspaceCode":"test", "userCode":"user", "locale":"en-US", "agentKind":"REACT",
        "model":{"modelCode":"MODEL-1", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://127.0.0.1:1/v1", "apiKey":"fixture", "modelName":"test", "maxTokens":128, "contextWindowTokens":131072},
        "variables":[], "skills":[], "tools":[], "executionScope":{}, "limits":{"maxSteps":2, "maxToolCalls":2, "maxParallelToolCalls":1}, "messages":[], "observations":[], "startedAt":"fixture"
    })).unwrap();
    context.messages.push(AgentMessage::user("question"));
    let result = harness.run(context).await.unwrap();
    assert_eq!(result.final_output, "authenticated answer");
    assert_eq!(client.calls.load(Ordering::SeqCst), 2);
    assert_eq!(client.refreshes.load(Ordering::SeqCst), 1);
    assert!(result.sdk_execution.is_some());
    assert!(host.close().await.is_success());
}

// The original 32k capacity remains a negative contract: newest tool output
// cannot be trimmed or replayed to make a second provider dispatch fit.
#[derive(Default)]
struct ProtectedTurnClient {
    calls: AtomicUsize,
}
impl ModelClient for ProtectedTurnClient {
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
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ModelResponse::tool_call(
                "echo",
                serde_json::json!({"value":"only"}),
            ))
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
#[tokio::test]
async fn newest_oversized_tool_turn_rejects_before_second_provider_without_repeating_effects() {
    use crate::domain::agent::{InProcessTool, ToolObservation, ToolRequest};
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            move |request: ToolRequest| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "x".repeat(32_000),
                })
            },
        )))
        .unwrap();
    let client = Arc::new(ProtectedTurnClient::default());
    let host = FrameworkHost::temporary(None).unwrap();
    host.bind_model_client(client.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(registry),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context: AgentRunContext = serde_json::from_value(serde_json::json!({
        "conversationCode":null, "runCode":fw::RunId::new_v4().to_string(), "workspaceCode":"test", "userCode":"user", "locale":"en-US", "agentKind":"REACT",
        "model":{"modelCode":"MODEL-1", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://127.0.0.1:1/v1", "apiKey":"fixture", "modelName":"test", "maxTokens":4096, "contextWindowTokens":32000},
        "variables":[], "skills":[], "tools":[{"toolName":"echo","description":"fixture", "inputSchema":{"type":"object"}, "skillName":null, "requiredVariableKeys":[]}],
        "executionScope":{}, "limits":{"maxSteps":3, "maxToolCalls":2, "maxParallelToolCalls":1}, "messages":[], "observations":[], "startedAt":"fixture"
    })).unwrap();
    context.messages.push(AgentMessage::user("question"));
    let error = harness.run(context).await.unwrap_err();
    assert!(
        error.to_string().contains("leave no summary capacity"),
        "{error}"
    );
    assert_eq!(
        client.calls.load(Ordering::SeqCst),
        1,
        "capacity rejection must precede another dispatch"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "completed side effects must never be replayed"
    );
    assert!(host.close().await.is_success());
}
