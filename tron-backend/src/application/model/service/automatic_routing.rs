//! Product-owned catalogue and source policy. Framework owns selection; provider
//! adapters retain credentials and execute the concrete model chosen here.
use super::*;
use crate::domain::model::{AutoRouteSources, AutoRouterKind, ModelInputCapability};
use crate::domain::model_provider::{
    ModelProviderCode, ModelSelection, ProviderCatalogModelStatus, ProviderCatalogModelType,
    ProviderConnectionStatus, ProviderModelQuality,
};
use eliteforge_agent_core as fw;

#[derive(Debug, Clone)]
pub struct AutomaticRouteModel {
    pub provider_id: String,
    pub provider_name: String,
    pub model_code: String,
    pub model_name: String,
    pub model_type: ModelType,
    pub context_window_tokens: Option<u32>,
    pub input_modalities: Vec<fw::Modality>,
    pub protocol: String,
    pub dimension: Option<usize>,
}
impl AutomaticRouteModel {
    pub fn eligible_for(&self, kind: AutoRouterKind) -> bool {
        (self.model_type == kind.model_type()
            || (kind == AutoRouterKind::TextRerank
                && self.model_type == ModelType::MultimodalRerank
                && self.input_modalities.contains(&fw::Modality::Text)))
            && (!matches!(
                kind,
                AutoRouterKind::TextEmbedding | AutoRouterKind::MultimodalEmbedding
            ) || self.dimension == Some(crate::shared::sqlite_extensions::TRON_VECTOR_INDEX_DIMENSION))
            && (kind != AutoRouterKind::Chat
                || (self.context_window_tokens.is_some_and(|n| n > 0)
                    && (!self.model_code.starts_with("PERSON_AI:")
                        || self.model_code.starts_with("PERSON_AI:FIXED:High:"))))
    }

    pub fn candidate(&self) -> fw::AutoRouteCandidate {
        let mut candidate =
            fw::AutoRouteCandidate::new(fw::ModelRef::new(&self.provider_id, &self.model_code));
        candidate.protocol = self.protocol.clone();
        candidate.context_window_tokens = self.context_window_tokens.map(|v| v as usize);
        candidate.input_modalities = self.input_modalities.clone();
        candidate
    }
}

impl ModelService {
    pub fn automatic_route_catalog(&self) -> Result<Vec<AutomaticRouteModel>, ModelError> {
        let mut result = Vec::new();
        for provider in self
            .list_configured_providers()
            .map_err(|_| ModelError::StorageFailure)?
        {
            if !provider.enabled {
                continue;
            }
            for model in provider.models.iter().filter(|m| m.enabled) {
                let kind = model.model_type.model_type();
                if !matches!(
                    kind,
                    ModelType::Text
                        | ModelType::TextEmbedding
                        | ModelType::MultimodalEmbedding
                        | ModelType::TextRerank
                        | ModelType::MultimodalRerank
                ) {
                    continue;
                }
                result.push(AutomaticRouteModel {
                    provider_id: provider.provider_id.clone(),
                    provider_name: provider.display_name.clone(),
                    model_code: model.model_code.clone().ok_or(ModelError::StorageFailure)?,
                    model_name: model.name.clone(),
                    model_type: kind,
                    context_window_tokens: model.context_window_tokens,
                    input_modalities: model
                        .input_capabilities
                        .iter()
                        .map(|c| match c {
                            ModelInputCapability::Text => fw::Modality::Text,
                            ModelInputCapability::Image => fw::Modality::Image,
                            ModelInputCapability::Audio => fw::Modality::Audio,
                            ModelInputCapability::Video => fw::Modality::Video,
                        })
                        .collect(),
                    protocol: format!(
                        "{:?}",
                        model
                            .model_type
                            .protocol(provider.protocol)
                            .ok_or(ModelError::InvalidProtocolModelType)?
                    ),
                    dimension: model.dimension.map(|d| d as usize),
                });
            }
        }
        let connected = self
            .providers
            .list_connections()
            .map_err(provider_error)?
            .iter()
            .any(|p| {
                matches!(
                    p.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                )
            });
        if connected {
            for model in self
                .providers
                .list_catalog_models(ModelProviderCode::PersonAi)
                .map_err(provider_error)?
            {
                if model.status != ProviderCatalogModelStatus::Available {
                    continue;
                }
                let (kind, code, inputs) = match model.catalog_model_type {
                    ProviderCatalogModelType::Chat => {
                        let Some(quality) = model.quality_type else {
                            continue;
                        };
                        (
                            ModelType::Text,
                            format!(
                                "PERSON_AI:FIXED:{}:{}",
                                quality.storage_value(),
                                model.selection_model_code()
                            ),
                            vec![fw::Modality::Text],
                        )
                    }
                    ProviderCatalogModelType::Embedding => {
                        let multi = model.tags.iter().any(|t| t == "multimodal_embedding");
                        if !multi && !model.tags.iter().any(|t| t == "text_embedding") {
                            continue;
                        }
                        (
                            if multi {
                                ModelType::MultimodalEmbedding
                            } else {
                                ModelType::TextEmbedding
                            },
                            format!("PERSON_AI:EMBEDDING:{}", model.model_code),
                            if multi {
                                vec![fw::Modality::Text, fw::Modality::Image]
                            } else {
                                vec![fw::Modality::Text]
                            },
                        )
                    }
                    ProviderCatalogModelType::Rerank => {
                        let Some(id) = model.model_id.as_ref().filter(|id| !id.is_empty()) else {
                            continue;
                        };
                        let multi = model.tags.iter().any(|t| t == "multimodal_rerank");
                        (
                            if multi {
                                ModelType::MultimodalRerank
                            } else {
                                ModelType::TextRerank
                            },
                            format!("PERSON_AI:RERANK:{id}"),
                            if multi {
                                vec![fw::Modality::Text, fw::Modality::Image]
                            } else {
                                vec![fw::Modality::Text]
                            },
                        )
                    }
                    _ => continue,
                };
                result.push(AutomaticRouteModel {
                    provider_id: "PersonAi".into(),
                    provider_name: "Person-AI".into(),
                    model_code: code,
                    model_name: model.model_name,
                    model_type: kind,
                    context_window_tokens: model.context_window_tokens,
                    input_modalities: inputs,
                    protocol: "personai".into(),
                    dimension: model.dimension,
                });
            }
        }
        // Provider instances may advertise the same logical embedding model.
        // Routing and credentials bind that logical identity, not an instance ID.
        result.sort_by(|a, b| {
            (&a.provider_id, &a.model_code, a.model_type)
                .cmp(&(&b.provider_id, &b.model_code, b.model_type))
                .then_with(|| (b.dimension == Some(1024)).cmp(&(a.dimension == Some(1024))))
        });
        result.dedup_by(|a, b| {
            a.provider_id == b.provider_id
                && a.model_code == b.model_code
                && a.model_type == b.model_type
        });
        Ok(result)
    }
    pub fn automatic_route_sources(
        &self,
    ) -> Result<std::collections::BTreeMap<String, AutoRouteSources>, ModelError> {
        self.providers
            .automatic_route_sources()
            .map_err(provider_error)
    }
    pub fn save_automatic_route_sources(
        &self,
        kind: AutoRouterKind,
        sources: AutoRouteSources,
    ) -> Result<(), AutomaticRoutingSettingsError> {
        if !sources.valid() {
            return Err(AutomaticRoutingSettingsError::InvalidSources);
        }
        let catalog = self
            .automatic_route_catalog()
            .map_err(|_| AutomaticRoutingSettingsError::Storage)?;
        let providers = self
            .list_configured_providers()
            .map_err(|_| AutomaticRoutingSettingsError::Storage)?;
        if sources
            .provider_ids
            .iter()
            .any(|id| id != "PersonAi" && !providers.iter().any(|p| &p.provider_id == id))
            || sources.model_codes.iter().any(|id| {
                !catalog
                    .iter()
                    .any(|m| &m.model_code == id && m.eligible_for(kind))
            })
        {
            return Err(AutomaticRoutingSettingsError::InvalidSources);
        }
        self.providers
            .save_automatic_route_sources(kind, sources)
            .map_err(|_| AutomaticRoutingSettingsError::Storage)
    }
    pub fn automatic_route_models(
        &self,
        kind: AutoRouterKind,
    ) -> Result<Vec<AutomaticRouteModel>, ModelError> {
        let sources = self
            .automatic_route_sources()?
            .remove(kind.code())
            .unwrap_or_default();
        Ok(self
            .automatic_route_catalog()?
            .into_iter()
            .filter(|m| m.eligible_for(kind) && sources.allows(&m.provider_id, &m.model_code))
            .collect())
    }
    /// Called before credentials are bound into an immutable run snapshot.
    pub fn automatic_chat_model_codes(
        &self,
    ) -> Result<std::collections::HashSet<String>, ModelError> {
        Ok(self
            .automatic_route_models(AutoRouterKind::Chat)?
            .into_iter()
            .map(|m| m.model_code)
            .collect())
    }
    pub fn route_automatic_model(
        &self,
        workspace: &str,
        kind: AutoRouterKind,
    ) -> Result<RuntimeModelConfig, ModelError> {
        let available = self
            .automatic_route_models(kind)?
            .into_iter()
            .map(|m| m.candidate())
            .collect::<Vec<_>>();
        let policy = Arc::new(eliteforge_agent_capability_auto_routing::StableAutoRouter);
        let routers = fw::AutoRouters {
            chat: Some(policy.clone()),
            text_generation: Some(policy.clone()),
            embedding: Some(policy.clone()),
            multimodal_embedding: Some(policy.clone()),
            rerank: Some(policy.clone()),
            multimodal_rerank: Some(policy),
            ..Default::default()
        };
        let operation = match kind {
            AutoRouterKind::Chat | AutoRouterKind::TextGeneration => fw::ModelOperation::Chat,
            AutoRouterKind::TextEmbedding | AutoRouterKind::MultimodalEmbedding => {
                fw::ModelOperation::Embedding
            }
            _ => fw::ModelOperation::Rerank,
        };
        let context = fw::AutoRouteContext {
            operation,
            stage: kind.code(),
            affinity: workspace,
            call_key: kind.code(),
            previous: None,
            transient_failures: 0,
        };
        let selected = match kind {
            AutoRouterKind::TextGeneration => routers.select_text_generation(&context, &available),
            AutoRouterKind::MultimodalEmbedding => {
                routers.select_multimodal_embedding(&context, &available)
            }
            AutoRouterKind::MultimodalRerank => {
                routers.select_multimodal_rerank(&context, &available)
            }
            _ => routers.select(&context, &available),
        }
        .map_err(|e| ModelError::RuntimeResolutionFailed(e.to_string()))?;
        let code = &selected.model;
        let config = match kind {
            AutoRouterKind::Chat | AutoRouterKind::TextGeneration => {
                let agent = if let Some(encoded) = code.strip_prefix("PERSON_AI:FIXED:") {
                    let (quality, code) = encoded
                        .split_once(':')
                        .ok_or(ModelError::InvalidProtocolModelType)?;
                    let quality = ProviderModelQuality::from_storage(quality)
                        .ok_or(ModelError::InvalidProtocolModelType)?;
                    if quality == ProviderModelQuality::High {
                        self.text_agent_model_config(workspace, Some(&selected.model))?
                    } else {
                        self.providers
                            .resolve_temporary_text_selection(
                                workspace,
                                &ModelSelection::person_ai_fixed(code, quality),
                            )
                            .map_err(provider_error)?
                    }
                } else {
                    self.text_agent_model_config(workspace, Some(code))?
                };
                agent.map(crate::application::model_router::runtime_from_agent)
            }
            AutoRouterKind::TextEmbedding | AutoRouterKind::MultimodalEmbedding => {
                if code.starts_with("PERSON_AI:") {
                    self.providers
                        .embedding_runtime_model_config(workspace, Some(code))
                        .map_err(provider_error)?
                } else {
                    self.auxiliary_runtime_model(Some(code), kind.model_type())?
                }
            }
            _ => {
                if code.starts_with("PERSON_AI:") {
                    self.providers
                        .rerank_runtime_model_config(workspace, Some(code))
                        .map_err(provider_error)?
                } else {
                    self.auxiliary_runtime_model(Some(code), kind.model_type())?
                }
            }
        };
        config
            .map(|mut config| {
                config.model_type = kind.model_type();
                config
            })
            .ok_or_else(|| {
                ModelError::RuntimeResolutionFailed("automatic route model is unavailable".into())
            })
    }
}
fn provider_error(error: crate::application::model_provider::ModelProviderError) -> ModelError {
    ModelError::RuntimeResolutionFailed(error.to_string())
}

#[derive(Debug, Clone, Copy)]
pub enum AutomaticRoutingSettingsError {
    InvalidSources,
    Storage,
}
impl AutomaticRoutingSettingsError {
    pub fn descriptor(self) -> ApplicationErrorDescriptor {
        match self {
            Self::InvalidSources => ApplicationErrorDescriptor {
                code: "101032012",
                message_key: "tron.automatic_router.invalid_sources",
            },
            Self::Storage => ApplicationErrorDescriptor {
                code: "101032013",
                message_key: "tron.automatic_router.storage_failed",
            },
        }
    }
}

#[cfg(test)]
mod rerank_capability_tests {
    use super::*;
    #[test]
    fn embedding_candidates_require_confirmed_index_dimension() {
        let mut model = AutomaticRouteModel { provider_id: "p".into(), provider_name: "P".into(), model_code: "e".into(), model_name: "E".into(), model_type: ModelType::TextEmbedding, context_window_tokens: None, input_modalities: vec![fw::Modality::Text], protocol: "p".into(), dimension: None };
        assert!(!model.eligible_for(AutoRouterKind::TextEmbedding));
        model.dimension = Some(768); assert!(!model.eligible_for(AutoRouterKind::TextEmbedding));
        model.dimension = Some(1024); assert!(model.eligible_for(AutoRouterKind::TextEmbedding));
        model.model_type = ModelType::MultimodalEmbedding; assert!(model.eligible_for(AutoRouterKind::MultimodalEmbedding));
        model.dimension = None; assert!(!model.eligible_for(AutoRouterKind::MultimodalEmbedding));
    }
    #[test]
    fn multimodal_reranker_is_text_candidate_only_when_it_accepts_text() {
        let mut model = AutomaticRouteModel { provider_id: "p".into(), provider_name: "P".into(), model_code: "m".into(), model_name: "M".into(), model_type: ModelType::MultimodalRerank, context_window_tokens: None, input_modalities: vec![fw::Modality::Text, fw::Modality::Image], protocol: "personai".into(), dimension: None };
        assert!(model.eligible_for(AutoRouterKind::TextRerank));
        assert!(model.eligible_for(AutoRouterKind::MultimodalRerank));
        model.input_modalities = vec![fw::Modality::Image];
        assert!(!model.eligible_for(AutoRouterKind::TextRerank));
        assert!(model.eligible_for(AutoRouterKind::MultimodalRerank));
        model.model_type = ModelType::TextRerank;
        assert!(!model.eligible_for(AutoRouterKind::MultimodalRerank));
    }
}
