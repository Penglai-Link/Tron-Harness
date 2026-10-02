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
    ) -> ObservedModelResult<Vec<f32>> {
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
        let mut request = fw::EmbeddingRequest::texts([input]);
        request.choice = fw::ModelChoice::model(reference);
        let jobs = match self.models.jobs.get().await {
            Ok(jobs) => jobs,
            Err(error) => {
                return ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let observed = jobs
            .embed_observed(
                service,
                request,
                limits(&model),
                config.max_retries.unwrap_or(0) as usize + 2,
            )
            .await;
        ObservedModelResult {
            result: observed.result.and_then(|response| {
                response
                    .vectors
                    .into_iter()
                    .next()
                    .map(|vector| vector.values)
                    .ok_or_else(|| fw::AgentError::Run("embedding response empty".into()))
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
        let jobs = match self.models.jobs.get().await {
            Ok(jobs) => jobs,
            Err(error) => {
                return ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        jobs.rerank_reported(
            service,
            request,
            limits(&model),
            config.max_retries.unwrap_or(0) as usize + 2,
        )
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
