use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::{ChatModelJob, ModelJobClient};
use eliteforge_agent_official_plugin_model_http as sdk;
use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    application::agent::{ModelClient, ModelStreamSink},
    domain::agent::{AgentModelConfig, ModelRequest, ModelResponse},
};

pub(crate) fn convert<T: serde::de::DeserializeOwned>(
    value: &impl serde::Serialize,
) -> Result<T, String> {
    serde_json::from_value(serde_json::to_value(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
pub(crate) fn sdk_config(config: &AgentModelConfig) -> Result<sdk::AgentModelConfig, String> {
    let mut config: sdk::AgentModelConfig = convert(config)?;
    if config.model_code.starts_with("PERSON_AI:") {
        config.api_path = None;
    }
    Ok(config)
}
pub(crate) fn error(error: fw::AgentError) -> String {
    match error.primary() {
        fw::AgentError::Model(error) => sdk::encode_model_error(error),
        _ => error.to_string(),
    }
}
pub(crate) fn limits(config: &AgentModelConfig) -> fw::RunLimits {
    fw::RunLimits {
        max_model_retries: config.max_retries.unwrap_or(0) as usize,
        max_model_calls_per_step: Some(config.max_retries.unwrap_or(0) as usize + 2),
        max_model_output_tokens: config.max_tokens.filter(|v| *v > 0).unwrap_or(4096),
        model_timeout: std::time::Duration::from_secs(30),
        run_timeout: std::time::Duration::from_secs(30),
        max_context_tokens: config.context_window_tokens.unwrap_or(128_000) as usize,
        ..Default::default()
    }
}

#[derive(Clone)]
pub struct HttpModelClient {
    pub(crate) factory: Arc<dyn sdk::ModelTransportFactory>,
    pub(crate) jobs: Arc<eliteforge_agent_framework::LazyModelJobClient>,
}
impl HttpModelClient {
    pub fn new(factory: Arc<dyn sdk::ModelTransportFactory>) -> Self {
        Self {
            factory,
            jobs: Arc::new(Default::default()),
        }
    }
    pub fn with_jobs(mut self, jobs: Arc<eliteforge_agent_framework::LazyModelJobClient>) -> Self {
        self.jobs = jobs;
        self
    }
    pub(crate) async fn jobs(&self) -> Result<&ModelJobClient, String> {
        self.jobs.get().await.map_err(error)
    }
    fn build_job(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        repair: Option<Arc<dyn eliteforge_agent_framework::ModelJobRepair>>,
        max_calls: usize,
    ) -> Result<ChatModelJob, fw::AgentError> {
        let fail = |message| fw::AgentError::InvalidConfiguration(message);
        let wire = sdk_config(config).map_err(fail)?;
        let mut input = sdk::request_to_framework(convert(request).map_err(fail)?)?;
        input.options.temperature = config.temperature;
        input.options.top_p = config.top_p;
        input.options.max_output_tokens = config.max_tokens.filter(|v| *v > 0);
        let factory = Arc::new(sdk::ScopedModelTransportFactory::new(
            self.factory.clone(),
            &convert(config).map_err(fail)?,
        )?);
        let descriptor = sdk::model_descriptor(&wire);
        let reference = fw::ModelRef::new("http-model", &wire.model_code);
        Ok(ChatModelJob {
            workflow: None,
            connection: fw::ModelConnection::new(
                &reference.connection,
                Arc::new(sdk::ProtocolChatModel {
                    config: wire.clone(),
                    factory,
                    streaming: false,
                }),
                vec![descriptor],
            )
            .with_protocol(sdk::model_protocol(&wire)),
            model: reference,
            request: input,
            limits: limits(config),
            stage: "host.stateless".into(),
            max_calls,
            repair,
        })
    }
    pub(crate) async fn job_observed(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        repair: Option<Arc<dyn eliteforge_agent_framework::ModelJobRepair>>,
        max_calls: usize,
        parent: Option<&fw::RunContext>,
    ) -> eliteforge_agent_framework::ObservedModelResult<ModelResponse> {
        let job = match self.build_job(config, request, repair, max_calls) {
            Ok(job) => job,
            Err(error) => {
                return eliteforge_agent_framework::ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let jobs = match self.jobs.get().await {
            Ok(jobs) => jobs,
            Err(error) => {
                return eliteforge_agent_framework::ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let observed = match parent {
            Some(parent) => jobs.execute_child_observed(job, parent).await,
            None => jobs.execute_observed(job).await,
        };
        eliteforge_agent_framework::ObservedModelResult {
            result: observed
                .result
                .and_then(|response| sdk::response_from_framework(response).map_err(Into::into))
                .and_then(|response| {
                    convert(&response).map_err(fw::AgentError::InvalidConfiguration)
                }),
            report: observed.report,
        }
    }
    pub(crate) async fn job(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        repair: Option<Arc<dyn eliteforge_agent_framework::ModelJobRepair>>,
        max_calls: usize,
    ) -> Result<ModelResponse, String> {
        self.job_observed(config, request, repair, max_calls, None)
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
            let mut input =
                sdk::request_from_framework(admitted).map_err(|e| sdk::encode_model_error(&e))?;
            // Continuation is prepared by the product turn state; every content block
            // and tool schema still comes from the SDK's final admitted request.
            // SDK regenerates the schema from the admitted descriptors.
            input.system_agent_session_id = product.system_agent_session_id.clone();
            input.previous_response_id = product.previous_response_id.clone();
            input.response_input_items = product.response_input_items.clone();
            let transport = self
                .factory
                .create(&convert(config)?)
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
                                if let Ok(event) = convert(&event) {
                                    sink(event)
                                }
                            }),
                        )
                        .await
                }
                None => client.generate(&sdk_config(config)?, &input).await,
            }
            .map_err(|e| sdk::encode_model_error(&e))?;
            convert(&response)
        })
    }

    fn bind_for_run(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Option<Arc<dyn ModelClient>>, String> {
        Ok(Some(Arc::new(Self {
            factory: Arc::new(
                sdk::ScopedModelTransportFactory::new(self.factory.clone(), &convert(config)?)
                    .map_err(|e| sdk::encode_model_error(&e))?,
            ),
            jobs: self.jobs.clone(),
        })))
    }
    fn authorization_scope(&self, config: &AgentModelConfig) -> Result<Option<String>, String> {
        self.factory
            .authorization_scope(&convert(config)?)
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
        Box::pin(async move { Ok(self.jobs.close_observed().await) })
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
            self.job_observed(
                config,
                request,
                None,
                config.max_retries.unwrap_or(0) as usize + 2,
                None,
            )
            .await
        })
    }
    fn generate_goal_judge_child<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        repair: Arc<dyn eliteforge_agent_framework::ModelJobRepair>,
        max_calls: usize,
        parent: &'a fw::RunContext,
    ) -> Pin<
        Box<
            dyn Future<Output = eliteforge_agent_framework::ObservedModelResult<ModelResponse>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.job_observed(config, request, Some(repair), max_calls, Some(parent))
                .await
        })
    }

    fn compact_memory<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        history: &'a [String],
        previous_summary: Option<&'a str>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_memory::MemoryCompactionResult, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let wire = sdk_config(config)?;
            let job = ChatModelJob {
                workflow: Some(Arc::new(MemoryWorkflow {
                    history: history.to_vec(),
                    previous: previous_summary.map(str::to_owned),
                })),
                connection: fw::ModelConnection::new(
                    "memory",
                    Arc::new(sdk::ProtocolChatModel {
                        config: wire.clone(),
                        factory: self.factory.clone(),
                        streaming: false,
                    }),
                    vec![sdk::model_descriptor(&wire)],
                ),
                model: fw::ModelRef::new("memory", &wire.model_code),
                request: fw::ChatRequest {
                    messages: vec![],
                    tools: vec![],
                    extensions: Default::default(),
                    options: Default::default(),
                },
                limits: fw::RunLimits {
                    max_model_calls_per_step: Some(17),
                    ..limits(config)
                },
                stage: "memory.compaction".into(),
                max_calls: 16,
                repair: None,
            };
            let response = self.jobs().await?.execute(job).await.map_err(error)?;
            eliteforge_agent_memory::MemoryCompactionResult::from_chat_response(response)
                .map_err(error)
        })
    }
    fn on_model_failure(&self, config: &AgentModelConfig, error: &fw::ModelError) {
        if let Ok(config) = convert(config) {
            self.factory.on_failure(&config, error);
        }
    }
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.job(
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
                .create(&convert(config)?)
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            let response = sdk::HttpProtocolClient::new(transport)
                .generate(&sdk_config(config)?, &convert(request)?)
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            convert(&response)
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
                .create(&convert(config)?)
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            let response = sdk::HttpProtocolClient::new(transport)
                .generate_stream(
                    &sdk_config(config)?,
                    &convert(request)?,
                    Arc::new(move |event| {
                        if let Ok(event) = convert(&event) {
                            sink(event);
                        }
                    }),
                )
                .await
                .map_err(|e| sdk::encode_model_error(&e))?;
            convert(&response)
        })
    }
    fn recover_authentication<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        error: &'a fw::ModelError,
    ) -> Pin<Box<dyn Future<Output = Result<bool, fw::ModelError>> + Send + 'a>> {
        Box::pin(async move {
            self.factory
                .recover_authentication(&convert(config).map_err(fw::ModelError::Fatal)?, error)
                .await
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            self.jobs.close().await;
        })
    }
    fn generate_with_repair<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        repair: Arc<dyn eliteforge_agent_framework::ModelJobRepair>,
        max_calls: usize,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move { self.job(config, request, Some(repair), max_calls).await })
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
        Ok(Arc::new(sdk::HttpModelTransport::new(
            self.client.clone(),
            &config.base_url,
            &config.api_key,
        )))
    }
}

struct MemoryWorkflow {
    history: Vec<String>,
    previous: Option<String>,
}
#[async_trait]
impl fw::ModelWorkflow for MemoryWorkflow {
    async fn run(
        &self,
        run: &fw::RunContext,
        models: &dyn fw::ChatExecutor,
        limits: &fw::RunLimits,
        model: &fw::ModelRef,
    ) -> Result<fw::ChatResponse, fw::AgentError> {
        let compactor = eliteforge_agent_memory::MemoryCompactor::new(
            model.clone(),
            eliteforge_agent_memory::CompressionOptions {
                max_input_tokens: limits
                    .max_context_tokens
                    .saturating_sub(limits.max_model_output_tokens as usize)
                    .max(1),
                max_summary_bytes: limits.max_model_output_tokens as usize,
                max_model_calls: 16,
            },
        )?;
        let result = compactor
            .compact_history_with_usage(
                &self.history,
                self.previous.as_deref(),
                run,
                models,
                limits,
            )
            .await?;
        Ok(result.into_chat_response())
    }
}

#[cfg(test)]
mod controlled_native_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[tokio::test]
    async fn native_image_and_attachment_generate_uses_controlled_sdk_transport() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut data = Vec::new();
            let mut buf = [0; 4096];
            let (end, len) = loop {
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                data.extend_from_slice(&buf[..n]);
                if let Some(end) = data.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]);
                    let len = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, len);
                }
            };
            while data.len() < end + len {
                let n = socket.read(&mut buf).await.unwrap();
                data.extend_from_slice(&buf[..n]);
            }
            let body: serde_json::Value = serde_json::from_slice(&data[end..end + len]).unwrap();
            assert!(body.to_string().contains("data:image/png;base64,aW1hZ2U="));
            assert!(body.to_string().contains("reference data"));
            let answer = r#"{"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":30,"completion_tokens":1}}"#;
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",answer.len()).as_bytes()).await.unwrap();
        });
        let config:AgentModelConfig=serde_json::from_value(serde_json::json!({"modelCode":"fixture","protocol":"CHAT_COMPLETIONS","baseUrl":url,"apiKey":"fixture","modelName":"fixture","maxTokens":32,"contextWindowTokens":12000})).unwrap();
        let request:ModelRequest=serde_json::from_value(serde_json::json!({"messages":[{"role":"user","parts":[{"type":"text","text":"inspect reference data"},{"type":"image","mime_type":"image/png","data":"aW1hZ2U="}]}],"tools":[]})).unwrap();
        let client = HttpModelClient::new(Arc::new(HttpModelProviderClient::new(
            sdk::ModelHttpClient::new(),
        )));
        let response = client.generate(&config, &request).await.unwrap();
        assert_eq!(response.output_text(), "ok");
        client.close().await;
        server.await.unwrap();
    }
}
