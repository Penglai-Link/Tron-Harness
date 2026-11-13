use async_trait::async_trait;
use eliteforge_agent_core as fw;

use eliteforge_agent_official_plugin_model_http as sdk;
use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    application::agent::{ModelClient, ModelStreamSink},
    domain::agent::{AgentModelConfig, ModelRequest, ModelResponse},
};

pub(crate) fn sdk_config(config: &AgentModelConfig) -> Result<sdk::AgentModelConfig, String> {
    let mut config = sdk::AgentModelConfig::from(config);
    if config.model_code.starts_with("PERSON_AI:") {
        config.api_path = None;
    }
    Ok(config)
}

pub(crate) fn native_request(admitted: &fw::ChatRequest) -> Result<sdk::ModelRequest, String> {
    let messages = admitted
        .messages
        .iter()
        .map(crate::application::agent::framework_runtime::decode_native_message)
        .collect::<Result<Vec<_>, _>>()?;
    let product = ModelRequest {
        messages,
        tools: Vec::new(),
        prepared_tool_schema: None,
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };
    // Translate the host's reserved records through both typed codecs. Never
    // rename arbitrary JSON keys or turn native tool identities into prose.
    let mut wire = admitted.clone();
    wire.messages = sdk::request_to_framework(product.into())
        .map_err(|e| e.to_string())?
        .messages;
    sdk::request_from_framework(&wire).map_err(|e| sdk::encode_model_error(&e))
}
pub(crate) fn error(error: fw::AgentError) -> String {
    match error.primary() {
        fw::AgentError::Model(error) => sdk::encode_model_error(error),
        _ => error.to_string(),
    }
}
pub(crate) fn limits(
    config: &AgentModelConfig,
    timeouts: &crate::config::ModelInvocationTimeouts,
    default_output_tokens: u32,
) -> fw::RunLimits {
    fw::RunLimits {
        max_model_retries: config.max_retries.unwrap_or(0) as usize,
        max_model_calls_per_step: Some(config.max_retries.unwrap_or(0) as usize + 2),
        max_model_output_tokens: config
            .max_tokens
            .filter(|v| *v > 0)
            .unwrap_or(default_output_tokens),
        model_timeout: std::time::Duration::from_secs(timeouts.call_timeout_seconds),
        run_timeout: std::time::Duration::from_secs(timeouts.operation_timeout_seconds),
        max_context_tokens: config.context_window_tokens.unwrap_or(128_000) as usize,
        ..Default::default()
    }
}

#[derive(Clone)]
pub struct HttpModelClient {
    pub(crate) factory: Arc<dyn sdk::ModelTransportFactory>,
    pub(crate) executor: Arc<eliteforge_agent_framework::ModelClient>,
    runtime_config: Option<crate::application::system_config::BackendRuntimeConfig>,
    authorization_bound: bool,
}
impl HttpModelClient {
    pub fn new(factory: Arc<dyn sdk::ModelTransportFactory>) -> Self {
        Self {
            factory,
            executor: Arc::default(),
            runtime_config: None,
            authorization_bound: false,
        }
    }
    pub fn with_runtime_config(
        mut self,
        runtime: crate::application::system_config::BackendRuntimeConfig,
    ) -> Self {
        self.runtime_config = Some(runtime);
        self
    }
    pub(crate) fn invocation_config(&self) -> crate::config::ModelInvocationConfig {
        self.runtime_config
            .as_ref()
            .map(|runtime| runtime.snapshot().model_invocation)
            .unwrap_or_default()
    }
    pub fn with_model_executor(
        mut self,
        executor: Arc<eliteforge_agent_framework::ModelClient>,
    ) -> Self {
        self.executor = executor;
        self
    }
    pub(crate) fn scoped_factory(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Arc<dyn sdk::ModelTransportFactory>, fw::AgentError> {
        if self.authorization_bound {
            return Ok(self.factory.clone());
        }
        Ok(Arc::new(sdk::ScopedModelTransportFactory::new(
            self.factory.clone(),
            &sdk::AgentModelConfig::from(config),
        )?))
    }
    fn chat_binding(
        &self,
        config: &AgentModelConfig,
    ) -> Result<(eliteforge_agent_framework::ModelClient, fw::ModelRef), fw::AgentError> {
        let wire = sdk_config(config).map_err(fw::AgentError::InvalidConfiguration)?;
        let reference = fw::ModelRef::new("http-model", &wire.model_code);
        let connection = fw::ModelConnection::new(
            &reference.connection,
            Arc::new(sdk::ProtocolChatModel {
                config: wire.clone(),
                factory: self.scoped_factory(config)?,
                streaming: false,
            }),
            vec![sdk::model_descriptor(&wire)],
        )
        .with_protocol(sdk::model_protocol(&wire));
        let runtime = eliteforge_agent_framework::ModelRuntime::new(
            eliteforge_agent_framework::ModelRuntimeUpdate::new("http-model", 0, vec![connection]),
        )?;
        Ok((self.executor.with_runtime(runtime), reference))
    }
    pub(crate) async fn invoke_observed(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        validator: Option<Arc<dyn fw::ModelResponseValidator>>,
        max_calls: usize,
        parent: Option<&fw::RunContext>,
    ) -> eliteforge_agent_framework::ObservedModelResult<ModelResponse> {
        let prepared = self.chat_binding(config).and_then(|(client, reference)| {
            let mut input = sdk::request_to_framework(sdk::ModelRequest::from(request))?;
            input.options.temperature = config.temperature;
            input.options.top_p = config.top_p;
            input.options.max_output_tokens = config.max_tokens.filter(|v| *v > 0);
            let invocation = self.invocation_config();
            let mut effective = limits(config, &invocation.text, invocation.default_output_tokens);
            effective.max_model_calls_per_step = Some(max_calls);
            let client = match parent {
                Some(parent) => {
                    client.with_options(fw::ModelInvocationOptions::for_run(parent, &effective))
                }
                None => client.with_options(fw::ModelInvocationOptions::for_call(&effective)?),
            };
            Ok((client, reference, input, effective))
        });
        let (client, reference, input, effective) = match prepared {
            Ok(value) => value,
            Err(error) => {
                return eliteforge_agent_framework::ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let observed = match validator {
            Some(validator) => {
                client
                    .chat_with_validator_observed(
                        &reference,
                        input,
                        &effective,
                        "host.stateless",
                        validator,
                    )
                    .await
            }
            None => {
                client
                    .chat_observed(&reference, input, &effective, "host.stateless")
                    .await
            }
        };
        eliteforge_agent_framework::ObservedModelResult {
            result: observed
                .result
                .and_then(|response| sdk::response_from_framework(response).map_err(Into::into))
                .map(crate::domain::agent::ModelResponse::from),
            report: observed.report,
        }
    }
    pub(crate) async fn invoke(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        validator: Option<Arc<dyn fw::ModelResponseValidator>>,
        max_calls: usize,
    ) -> Result<ModelResponse, String> {
        self.invoke_observed(config, request, validator, max_calls, None)
            .await
            .into_result()
            .map_err(error)
    }
}
impl ModelClient for HttpModelClient {
    fn generate_native_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        admitted: &'a fw::ChatRequest,
        product: &'a ModelRequest,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut input = native_request(admitted)?;
            // Continuation is prepared by the product turn state; every content block
            // and tool schema still comes from the SDK's final admitted request.
            // SDK regenerates the schema from the admitted descriptors.
            input.system_agent_session_id = product.system_agent_session_id.clone();
            input.previous_response_id = product.previous_response_id.clone();
            input.response_input_items = product.response_input_items.clone();
            let transport = self
                .factory
                .create(&sdk::AgentModelConfig::from(config))
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            let client = sdk::HttpProtocolClient::new(transport);
            let response = match sink {
                Some(sink) => {
                    client
                        .generate_stream(
                            &sdk_config(config)?,
                            &input,
                            Arc::new(move |event| {
                                sink(crate::domain::agent::ModelStreamEvent::from(event));
                            }),
                        )
                        .await
                }
                None => client.generate(&sdk_config(config)?, &input).await,
            }
            .map_err(|e| sdk::encode_model_error(&e))?;
            Ok(crate::domain::agent::ModelResponse::from(response))
        })
    }

    fn bind_for_run(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Option<Arc<dyn ModelClient>>, String> {
        Ok(Some(Arc::new(Self {
            factory: Arc::new(
                sdk::ScopedModelTransportFactory::new(
                    self.factory.clone(),
                    &sdk::AgentModelConfig::from(config),
                )
                .map_err(|e| sdk::encode_model_error(&e))?,
            ),
            executor: self.executor.clone(),
            runtime_config: self.runtime_config.clone(),
            authorization_bound: true,
        })))
    }
    fn bind_models_for_run(
        &self,
        configs: &[AgentModelConfig],
    ) -> Result<Vec<(Option<String>, Option<Arc<dyn ModelClient>>)>, String> {
        let mut person_ai: Option<(Option<String>, Option<Arc<dyn ModelClient>>)> = None;
        configs
            .iter()
            .map(|config| {
                let is_person_ai = config.model_code.starts_with("PERSON_AI:");
                if is_person_ai && let Some(binding) = &person_ai {
                    return Ok(binding.clone());
                }
                let factory = self.scoped_factory(config).map_err(error)?;
                let scope = factory
                    .authorization_scope(&sdk::AgentModelConfig::from(config))
                    .map_err(|e| sdk::encode_model_error(&e))?;
                let client: Arc<dyn ModelClient> = Arc::new(Self {
                    factory,
                    executor: self.executor.clone(),
                    runtime_config: self.runtime_config.clone(),
                    authorization_bound: true,
                });
                let binding = (scope, Some(client));
                if is_person_ai {
                    person_ai = Some(binding.clone());
                }
                Ok(binding)
            })
            .collect()
    }
    fn authorization_scope(&self, config: &AgentModelConfig) -> Result<Option<String>, String> {
        self.factory
            .authorization_scope(&sdk::AgentModelConfig::from(config))
            .map_err(|e| sdk::encode_model_error(&e))
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
        Box::pin(async move { Ok(self.executor.close_observed().await) })
    }
    fn generate_observed<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = eliteforge_agent_framework::ObservedModelResult<ModelResponse>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.invoke_observed(
                config,
                request,
                None,
                config.max_retries.unwrap_or(0) as usize + 2,
                None,
            )
            .await
        })
    }
    fn on_model_failure(&self, config: &AgentModelConfig, error: &fw::ModelError) {
        self.factory
            .on_failure(&sdk::AgentModelConfig::from(config), error);
    }
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.invoke(
                config,
                request,
                None,
                config.max_retries.unwrap_or(0) as usize + 2,
            )
            .await
        })
    }
    fn generate_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let transport = self
                .factory
                .create(&sdk::AgentModelConfig::from(config))
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            let response = sdk::HttpProtocolClient::new(transport)
                .generate(&sdk_config(config)?, &sdk::ModelRequest::from(request))
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            Ok(crate::domain::agent::ModelResponse::from(response))
        })
    }
    fn generate_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let transport = self
                .factory
                .create(&sdk::AgentModelConfig::from(config))
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            let response = sdk::HttpProtocolClient::new(transport)
                .generate_stream(
                    &sdk_config(config)?,
                    &sdk::ModelRequest::from(request),
                    Arc::new(move |event| {
                        sink(crate::domain::agent::ModelStreamEvent::from(event));
                    }),
                )
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            Ok(crate::domain::agent::ModelResponse::from(response))
        })
    }
    fn recover_authentication<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        error: &'a fw::ModelError,
    ) -> Pin<Box<dyn Future<Output = Result<bool, fw::ModelError>> + Send + 'a>> {
        Box::pin(async move {
            self.factory
                .recover_authentication(&sdk::AgentModelConfig::from(config), error)
                .await
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            self.executor.close().await;
        })
    }
    fn generate_with_repair<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        repair: Arc<dyn eliteforge_agent_core::ModelResponseValidator>,
        max_calls: usize,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move { self.invoke(config, request, Some(repair), max_calls).await })
    }
}

#[derive(Debug, Clone)]
pub struct HttpModelProviderClient {
    client: sdk::ModelHttpClient,
}
impl HttpModelProviderClient {
    pub fn new(client: sdk::ModelHttpClient) -> Self {
        Self { client }
    }
}
#[async_trait]
impl sdk::ModelTransportFactory for HttpModelProviderClient {
    async fn create(
        &self,
        config: &sdk::AgentModelConfig,
    ) -> Result<Arc<dyn sdk::ModelTransport>, fw::ModelError> {
        Ok(Arc::new(
            sdk::HttpModelTransport::new(self.client.clone(), &config.base_url, &config.api_key)
                .with_headers(&config.request_headers)?,
        ))
    }
}
