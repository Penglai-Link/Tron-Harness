//! Project authorized endpoint bindings into generic Framework registrations.
use super::*;
use sdk::ModelTransportFactory as _;
impl HttpLlmRuntimeClient {
    pub(super) fn auxiliary_pool(
        &self,
        config: &RuntimeModelConfig,
        dimensions: Option<usize>,
        rerank: bool,
    ) -> Result<
        (
            Arc<eliteforge_agent_framework::ModelRuntime>,
            fw::ModelChoice,
            Vec<Arc<sdk::HttpRerankModel>>,
        ),
        fw::AgentError,
    > {
        let legacy;
        let endpoints = if let Some(route) = &config.route {
            &route.candidates
        } else {
            legacy = vec![crate::application::model::RuntimeModelEndpoint {
                provider_id: if rerank {
                    "http-rerank"
                } else {
                    "http-embedding"
                }
                .into(),
                model_code: config.model_code.clone(),
                input_modalities: vec![fw::Modality::Text],
                dimension: dimensions,
                config: config.clone(),
            }];
            &legacy
        };
        let mut update =
            eliteforge_agent_framework::ModelRuntimeUpdate::new("auxiliary-route", 0, vec![]);
        let mut adapters = vec![];
        for endpoint in endpoints {
            if dimensions.is_some_and(|d| endpoint.dimension.is_some_and(|declared| d != declared))
            {
                continue;
            }
            let model = runtime_to_agent_model_config(&endpoint.config);
            let wire = sdk_config(&model).map_err(fw::AgentError::InvalidConfiguration)?;
            let factory = Arc::new(sdk::ScopedModelTransportFactory::new(
                self.models.factory.clone(),
                &wire,
            )?);
            let reference = fw::ModelRef::new(&endpoint.provider_id, &model.model_code);
            if rerank {
                let chat_instruction = match self.prompts.as_ref() {
                    Some(prompts) => prompts
                        .runtime_prompt_text(
                            "model.chat-rerank",
                            "en-US",
                            &std::collections::HashMap::new(),
                        )
                        .map_err(|error| fw::AgentError::InvalidConfiguration(error.to_string()))?,
                    None => None,
                };
                let adapter = Arc::new(sdk::HttpRerankModel {
                    config: wire,
                    factory,
                    chat_fallback: false,
                    chat_instruction,
                });
                update.rerank.push(
                    fw::RerankModelService::new(reference, adapter.clone())
                        .identity(&endpoint.model_code, &endpoint.provider_id)
                        .capabilities(fw::OperationCapabilities {
                            input: endpoint
                                .input_modalities
                                .iter()
                                .copied()
                                .filter(|m| matches!(m, fw::Modality::Text | fw::Modality::Image))
                                .collect(),
                            ..Default::default()
                        }),
                );
                adapters.push(adapter);
            } else {
                let supported = factory.embedding_input_modalities(&wire)?;
                let inputs = if config.route.is_some() {
                    endpoint
                        .input_modalities
                        .iter()
                        .copied()
                        .filter(|m| supported.contains(m))
                        .collect()
                } else {
                    supported
                };
                let service = fw::EmbeddingModelService::new(
                    reference.clone(),
                    Arc::new(sdk::HttpEmbeddingModel {
                        config: wire,
                        factory,
                    }),
                )
                .identity(&endpoint.model_code, &endpoint.provider_id)
                .capabilities(fw::OperationCapabilities {
                    input: inputs,
                    ..Default::default()
                });
                update.embedding.push(service);
                if let Some(dimensions) = dimensions.or(endpoint.dimension) {
                    update.spaces.push(fw::EmbeddingSpace {
                        model: reference,
                        logical_model_code: config
                            .route
                            .as_ref()
                            .filter(|route| route.shared_identity)
                            .map(|route| route.model_code.clone()),
                        revision: config
                            .route
                            .as_ref()
                            .filter(|route| route.shared_identity)
                            .map_or_else(|| "v1".into(), |route| route.model_code.clone()),
                        dimensions,
                        preprocessing: "knowledge-base-local-v1".into(),
                    });
                }
            }
        }
        if update.embedding.is_empty() && update.rerank.is_empty() {
            return Err(fw::AgentError::InvalidConfiguration(
                "no compatible endpoint in auxiliary pool".into(),
            ));
        }
        let choice = if let Some(route) = &config.route {
            fw::ModelChoice {
                model_code: Some(route.model_code.clone()),
                required_modalities: route.required_modalities.clone(),
                ..Default::default()
            }
        } else {
            let reference = if rerank {
                &update.rerank[0].model
            } else {
                &update.embedding[0].model
            };
            if config.model_code.contains(":ROUTE:") {
                fw::ModelChoice::default()
            } else {
                fw::ModelChoice::model(reference.clone())
            }
        };
        Ok((
            eliteforge_agent_framework::ModelRuntime::new(update)?,
            choice,
            adapters,
        ))
    }
    pub(super) fn pool_routers(&self, config: &RuntimeModelConfig) -> fw::AutoRouters {
        if config.route.is_some() {
            fw::AutoRouters {
                embedding: Some(self.provider_rotation.clone()),
                rerank: Some(self.provider_rotation.clone()),
                ..Default::default()
            }
        } else {
            self.auto_routers.clone()
        }
    }
}
