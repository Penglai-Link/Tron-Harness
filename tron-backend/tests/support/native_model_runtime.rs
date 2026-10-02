//! Real loopback production-model paths, using the same host/context fixture.
use super::*;
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_model_http as sdk;
use tron_backend::adapters::outbound::model::HttpModelClient;

#[tokio::test]
async fn native_runtime_real_model_identity_protocol_and_output_reach_main_http() {
    let server = ProviderTcpServer::new(vec![provider_answer("native answer")], false).await;
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = base_context("native-main-identity");
    context.model.base_url = server.url.clone();
    context.model.model_code = "authorized-real-model".into();
    context.model.max_tokens = Some(64);
    let result = agent.run(context).await.unwrap();
    assert_eq!(result.final_output, "native answer");
    assert_eq!(result.sdk_execution.as_ref().unwrap().model_calls, 1);
    let body = server.bodies();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["model"], "gpt-test");
    assert_eq!(body[0]["max_tokens"], 64);
    assert_eq!(body[0]["temperature"], 0.2);
    server.drained().await;
    host.close().await;
}
#[tokio::test]
async fn stateless_declared_window_minus_output_rejects_before_real_http() {
    let server = ProviderTcpServer::new(vec![provider_answer("unused")], false).await;
    let client = real_provider_client();
    let mut config = base_context("capacity").model;
    config.base_url = server.url.clone();
    config.context_window_tokens = Some(256);
    config.max_tokens = Some(128);
    let request = ModelRequest {
        messages: vec![AgentMessage::user("x".repeat(200))],
        tools: vec![],
        prepared_tool_schema: None,
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };
    let observed = client.generate_observed(&config, &request).await;
    let error = observed.result.unwrap_err().to_string();
    assert!(
        error.contains("budget") || error.contains("capacity"),
        "{error}"
    );
    assert_eq!(observed.report.calls, 0);
    assert_eq!(observed.report.ledger_usage, fw::Usage::default());
    assert_eq!(server.bodies().len(), 0);
    assert!(client.close_observed().await.unwrap().is_success());
}
#[tokio::test]
async fn stateless_observed_unknown_usage_retry_partial_and_complete_metrics_are_distinct() {
    for (label, responses, expected_calls, complete) in [
        ("known", vec![provider_answer("ok")], 1, true),
        (
            "unknown",
            vec![
                json!({"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}]}),
            ],
            1,
            false,
        ),
        (
            "retry",
            vec![
                json!({"_http_status":503,"_http_body":{"error":{"message":"retry"}}}),
                provider_answer("ok"),
            ],
            2,
            false,
        ),
    ] {
        let server = ProviderTcpServer::new(responses, false).await;
        let client = real_provider_client();
        let mut config = base_context(label).model;
        config.base_url = server.url.clone();
        config.max_retries = Some(1);
        let request = ModelRequest {
            messages: vec![AgentMessage::user("hello")],
            tools: vec![],
            prepared_tool_schema: None,
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let observed = client.generate_observed(&config, &request).await;
        assert_eq!(observed.result.unwrap().output_text(), "ok");
        assert_eq!(observed.report.calls, expected_calls);
        assert_eq!(server.bodies().len(), expected_calls);
        assert_eq!(observed.report.provider_usage.is_some(), complete);
        assert!(observed.report.ledger_usage.input_tokens > 0);
        if complete {
            assert_eq!(
                observed
                    .report
                    .provider_metrics
                    .unwrap()
                    .cached_input_tokens,
                Some(16)
            );
        } else {
            assert!(observed.report.provider_metrics.is_none());
        }
        server.drained().await;
        assert!(client.close_observed().await.unwrap().is_success());
    }
}
struct AccountFactory {
    account: Arc<AtomicUsize>,
    client: sdk::ModelHttpClient,
}
#[async_trait]
impl sdk::ModelTransportFactory for AccountFactory {
    fn authorization_scope(
        &self,
        _: &sdk::AgentModelConfig,
    ) -> Result<Option<String>, fw::ModelError> {
        Ok(Some(format!(
            "account-{}",
            self.account.load(Ordering::SeqCst)
        )))
    }
    async fn create(
        &self,
        config: &sdk::AgentModelConfig,
    ) -> Result<Arc<dyn sdk::ModelTransport>, fw::ModelError> {
        Ok(Arc::new(sdk::HttpModelTransport::new(
            self.client.clone(),
            &config.base_url,
            "fixture",
        )))
    }
}
#[tokio::test]
async fn bound_model_client_rejects_account_switch_before_first_send() {
    let server = ProviderTcpServer::new(vec![provider_answer("never")], false).await;
    let account = Arc::new(AtomicUsize::new(1));
    let client = HttpModelClient::new(Arc::new(AccountFactory {
        account: account.clone(),
        client: sdk::ModelHttpClient::new(),
    }));
    let mut config = base_context("account-binding").model;
    config.base_url = server.url.clone();
    let bound = client.bind_for_run(&config).unwrap().unwrap();
    account.store(2, Ordering::SeqCst);
    let request = ModelRequest {
        messages: vec![AgentMessage::user("account A private input")],
        tools: vec![],
        prepared_tool_schema: None,
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };
    let observed = bound.generate_observed(&config, &request).await;
    assert!(
        matches!(observed.result.unwrap_err().primary(),fw::AgentError::Model(fw::ModelError::Provider(error)) if error.category==fw::ProviderErrorCategory::AccountChanged)
    );
    assert_eq!(server.bodies().len(), 0);
    assert_eq!(observed.report.calls, 0);
    assert!(bound.close_observed().await.unwrap().is_success());
}
struct LegacyOnly;
impl ModelClient for LegacyOnly {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async { panic!("unsupported defaults must never enter a new controlled job") })
    }
    fn close<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}
#[tokio::test]
async fn legacy_model_defaults_fail_explicitly_instead_of_nesting_or_faking_lifecycle() {
    let client = LegacyOnly;
    let config = base_context("defaults").model;
    let request = ModelRequest {
        messages: vec![],
        tools: vec![],
        prepared_tool_schema: None,
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };
    assert!(
        client
            .generate_once(&config, &request)
            .await
            .unwrap_err()
            .contains("explicitly")
    );
    assert!(client.close_observed().await.is_err());
    assert!(
        client
            .generate_observed(&config, &request)
            .await
            .result
            .is_err()
    );
}
