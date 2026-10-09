//! Observation is a returned SDK report, not an extra statistics pipeline.
use super::*;
use eliteforge_agent_framework::{ModelExecutionReport, ObservedModelResult};
fn failure<T>(message: impl Into<String>) -> ObservedModelResult<T> {
    ObservedModelResult {
        result: Err(fw::AgentError::InvalidConfiguration(message.into())),
        report: ModelExecutionReport::default(),
    }
}
impl HttpLlmRuntimeClient {
    pub(super) async fn observed_generate(
        &self,
        config: &RuntimeModelConfig,
        prompt: &str,
    ) -> ObservedModelResult<String> {
        if config.model_type != ModelType::Text {
            return failure("text generation requires TEXT model");
        }
        let request = ModelRequest {
            messages: vec![AgentMessage::user(prompt)],
            tools: vec![],
            prepared_tool_schema: None,
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let observed = self
            .models
            .generate_observed(&runtime_to_agent_model_config(config), &request)
            .await;
        ObservedModelResult {
            result: observed.result.and_then(|response| {
                if response.finish == ModelFinish::Final {
                    Ok(response.output_text())
                } else {
                    Err(fw::AgentError::Run(
                        "text generation unexpectedly requested tool calls".into(),
                    ))
                }
            }),
            report: observed.report,
        }
    }
    pub(super) async fn observed_embed(
        &self,
        config: &RuntimeModelConfig,
        input: &str,
    ) -> ObservedModelResult<(Vec<f32>, Option<String>)> {
        self.observed_embed_batch(config, &[input.to_owned()], None)
            .await
            .map(|response| {
                (
                    response
                        .vectors
                        .into_iter()
                        .next()
                        .expect("validated nonempty batch")
                        .values,
                    response.provider_model,
                )
            })
    }
    pub(super) async fn observed_embed_batch(
        &self,
        config: &RuntimeModelConfig,
        inputs: &[String],
        dimensions: Option<usize>,
    ) -> ObservedModelResult<fw::EmbeddingResponse> {
        if config.model_type != ModelType::Embedding {
            return failure("embedding requires EMBEDDING model");
        }
        let model = runtime_to_agent_model_config(config);
        let wire = match sdk_config(&model) {
            Ok(config) => config,
            Err(error) => return failure(error),
        };
        let factory =
            match sdk::ScopedModelTransportFactory::new(self.models.factory.clone(), &wire) {
                Ok(factory) => Arc::new(factory),
                Err(error) => {
                    return ObservedModelResult {
                        result: Err(error.into()),
                        report: Default::default(),
                    };
                }
            };
        let reference = fw::ModelRef::new("http-embedding", &model.model_code);
        let service = fw::EmbeddingModelService::new(
            reference.clone(),
            Arc::new(sdk::HttpEmbeddingModel {
                config: wire,
                factory,
            }),
        );
        let mut request = fw::EmbeddingRequest::texts(inputs.iter().cloned());
        request.dimensions = dimensions;
        request.choice = fw::ModelChoice::model(reference);
        let mut update =
            eliteforge_agent_framework::ModelRuntimeUpdate::new("http-embedding", 0, vec![]);
        update.embedding.push(service);
        let runtime = match eliteforge_agent_framework::ModelRuntime::new(update) {
            Ok(runtime) => runtime,
            Err(error) => {
                return ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let config = self.models.invocation_config().embedding;
        let effective = limits(&model, &config.timeouts(), self.models.invocation_config().default_output_tokens);
        let mut options = match fw::ModelInvocationOptions::for_call(&effective) {
            Ok(options) => options,
            Err(error) => return failure(error.to_string()),
        };
        options.max_batch_size = config.max_batch_inputs;
        let observed = self
            .models
            .executor
            .with_runtime(runtime)
            .with_options(options)
            .embed_observed(request, &effective)
            .await;
        ObservedModelResult {
            result: observed.result.and_then(|response| {
                response.validate(inputs.len(), dimensions)?;
                Ok(response)
            }),
            report: observed.report,
        }
    }
    pub(super) async fn observed_rerank(
        &self,
        config: &RuntimeModelConfig,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> ObservedModelResult<Vec<LlmRuntimeRerankHit>> {
        if config.model_type != ModelType::Rerank {
            return failure("rerank requires RERANK model");
        }
        if top_n == Some(0) {
            return failure("rerank top_n must be positive");
        }
        if documents.is_empty() {
            return ObservedModelResult {
                result: Ok(vec![]),
                report: Default::default(),
            };
        }
        let model = runtime_to_agent_model_config(config);
        let wire = match sdk_config(&model) {
            Ok(config) => config,
            Err(error) => return failure(error),
        };
        let factory =
            match sdk::ScopedModelTransportFactory::new(self.models.factory.clone(), &wire) {
                Ok(factory) => Arc::new(factory),
                Err(error) => {
                    return ObservedModelResult {
                        result: Err(error.into()),
                        report: Default::default(),
                    };
                }
            };
        let reference = fw::ModelRef::new("http-rerank", &model.model_code);
        let service = fw::RerankModelService::new(
            reference.clone(),
            Arc::new(sdk::HttpRerankModel {
                config: wire,
                factory,
                chat_fallback: model.model_code.starts_with("PERSON_AI:"),
            }),
        );
        let request = fw::RerankRequest {
            choice: fw::ModelChoice::model(reference),
            query: query.into(),
            documents: documents.iter().cloned().map(Into::into).collect(),
            top_n: top_n.map(|n| n.min(documents.len())),
        };
        let mut update =
            eliteforge_agent_framework::ModelRuntimeUpdate::new("http-rerank", 0, vec![]);
        update.rerank.push(service);
        let runtime = match eliteforge_agent_framework::ModelRuntime::new(update) {
            Ok(runtime) => runtime,
            Err(error) => {
                return ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let invocation = self.models.invocation_config();
        let effective = limits(&model, &invocation.rerank, invocation.default_output_tokens);
        let options = match fw::ModelInvocationOptions::for_call(&effective) {
            Ok(options) => options,
            Err(error) => return failure(error.to_string()),
        };
        self.models
            .executor
            .with_runtime(runtime)
            .with_options(options)
            .rerank_observed(request, &effective)
            .await
            .map(|response| {
                response
                    .results
                    .into_iter()
                    .map(|hit| LlmRuntimeRerankHit {
                        document_index: hit.index,
                        score: hit.score,
                    })
                    .collect()
            })
    }
}
