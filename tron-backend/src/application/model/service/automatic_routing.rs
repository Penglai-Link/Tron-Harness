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
    pub binding_id: String,
    pub shared_identity: bool,
    pub model_name: String,
    pub model_type: ModelType,
    pub context_window_tokens: Option<u32>,
    pub input_modalities: Vec<fw::Modality>,
    pub protocol: String,
    pub dimension: Option<usize>,
}
impl AutomaticRouteModel {
    pub fn eligible_for(&self, kind: AutoRouterKind) -> bool {
        let family = match kind {
            AutoRouterKind::Embedding => matches!(
                self.model_type,
                ModelType::TextEmbedding | ModelType::MultimodalEmbedding
            ),
            AutoRouterKind::Rerank => matches!(
                self.model_type,
                ModelType::TextRerank | ModelType::MultimodalRerank
            ),
            _ => self.model_type == kind.model_type(),
        };
        family
            && (kind != AutoRouterKind::SpeechRecognition
                || matches!(
                    self.protocol.as_str(),
                    "personai" | "OpenAiCompatible" | "Responses" | "Gemini"
                ))
            && (kind != AutoRouterKind::Chat
                || (self.context_window_tokens.is_some_and(|n| n > 0)
                    && (!self.binding_id.starts_with("PERSON_AI:")
                        || self.binding_id.starts_with("PERSON_AI:FIXED:High:"))))
    }

    pub fn allowed_by(&self, sources: &AutoRouteSources) -> bool {
        sources.allows(&self.provider_id, &self.model_code)
            || sources.allows(&self.provider_id, &self.binding_id)
    }
    pub fn candidate(&self) -> fw::AutoRouteCandidate {
        let mut candidate =
            fw::AutoRouteCandidate::new(fw::ModelRef::new(&self.provider_id, &self.binding_id));
        candidate.model_code = self.model_code.clone();
        candidate.provider_id = self.provider_id.clone();
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
                        | ModelType::SpeechRecognition
                        | ModelType::RealtimeSpeech
                        | ModelType::Image
                ) {
                    continue;
                }
                result.push(AutomaticRouteModel {
                    provider_id: provider.provider_id.clone(),
                    provider_name: provider.display_name.clone(),
                    model_code: model
                        .logical_model_code
                        .clone()
                        .or_else(|| model.model_code.clone())
                        .ok_or(ModelError::StorageFailure)?,
                    binding_id: model.model_code.clone().ok_or(ModelError::StorageFailure)?,
                    shared_identity: model.logical_model_code.is_some(),
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
                    ProviderCatalogModelType::Image => (
                        ModelType::Image,
                        format!(
                            "PERSON_AI:IMAGE:{}",
                            model.model_id.as_deref().unwrap_or(&model.model_code)
                        ),
                        vec![fw::Modality::Text, fw::Modality::Image],
                    ),
                    // Person-AI media_type=2 declares STT, not a realtime
                    // conversation model. Preserve each actual catalogue identity.
                    ProviderCatalogModelType::Speech => {
                        let Some(id) = model.model_id.as_deref().filter(|id| !id.is_empty()) else {
                            continue;
                        };
                        (
                            ModelType::SpeechRecognition,
                            format!("PERSON_AI:SPEECH:{id}"),
                            vec![fw::Modality::Audio],
                        )
                    }
                    _ => continue,
                };
                result.push(AutomaticRouteModel {
                    provider_id: "PersonAi".into(),
                    provider_name: "Person-AI".into(),
                    binding_id: code.clone(),
                    shared_identity: false,
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
                && a.binding_id == b.binding_id
                && a.model_type == b.model_type
        });
        Ok(result)
    }
    pub fn automatic_route_sources(
        &self,
    ) -> Result<std::collections::BTreeMap<String, AutoRouteSources>, ModelError> {
        let mut sources = self
            .providers
            .automatic_route_sources()
            .map_err(provider_error)?;
        // Preserve the admitted legacy pools when consolidating two policies.
        // An absent old policy had the old default; explicit empty selections stay empty.
        let catalog = self.automatic_route_catalog()?;
        for (kind, text, multi) in [
            (
                AutoRouterKind::Embedding,
                "TextEmbedding",
                "MultimodalEmbedding",
            ),
            (AutoRouterKind::Rerank, "TextRerank", "MultimodalRerank"),
        ] {
            if sources.contains_key(kind.code())
                || (!sources.contains_key(text) && !sources.contains_key(multi))
            {
                continue;
            }
            let text_sources = sources.remove(text).unwrap_or_default();
            let multi_sources = sources.remove(multi).unwrap_or_default();
            let mut codes = catalog
                .iter()
                .filter(|model| model.eligible_for(kind))
                .filter(|model| {
                    let old = if matches!(
                        model.model_type,
                        ModelType::MultimodalEmbedding | ModelType::MultimodalRerank
                    ) {
                        &multi_sources
                    } else {
                        &text_sources
                    };
                    old.allows(&model.provider_id, &model.binding_id)
                        || old.allows(&model.provider_id, &model.model_code)
                })
                .map(|model| model.binding_id.clone())
                .collect::<Vec<_>>();
            codes.sort();
            codes.dedup();
            sources.insert(
                kind.code().into(),
                AutoRouteSources {
                    mode: crate::domain::model::AutoRouteSourceMode::SelectedModels,
                    provider_ids: vec![],
                    model_codes: codes,
                },
            );
        }
        Ok(sources)
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
                    .any(|m| (&m.model_code == id || &m.binding_id == id) && m.eligible_for(kind))
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
            .filter(|m| m.eligible_for(kind) && m.allowed_by(&sources))
            .collect())
    }
    /// Called before credentials are bound into an immutable run snapshot.
    pub fn automatic_chat_model_codes(
        &self,
    ) -> Result<std::collections::HashSet<String>, ModelError> {
        Ok(self
            .automatic_route_models(AutoRouterKind::Chat)?
            .into_iter()
            .map(|m| m.binding_id)
            .collect())
    }
    pub fn route_automatic_model(
        &self,
        workspace: &str,
        kind: AutoRouterKind,
    ) -> Result<RuntimeModelConfig, ModelError> {
        if matches!(
            kind,
            AutoRouterKind::SpeechRecognition
                | AutoRouterKind::Realtime
                | AutoRouterKind::ImageGeneration
        ) {
            let model = self.select_automatic_media_model(kind)?;
            return self
                .auxiliary_runtime_model(Some(&model.binding_id), kind.model_type())?
                .ok_or(ModelError::InvalidProtocolModelType);
        }
        if matches!(kind, AutoRouterKind::Embedding | AutoRouterKind::Rerank) {
            return self.route_auxiliary_model(workspace, kind, &fw::ModelChoice::default(), None);
        }
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
            AutoRouterKind::Embedding => fw::ModelOperation::Embedding,
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
            AutoRouterKind::Embedding => {
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
impl ModelService {
    pub fn route_auxiliary_model(
        &self,
        workspace: &str,
        kind: AutoRouterKind,
        choice: &fw::ModelChoice,
        dimensions: Option<usize>,
    ) -> Result<RuntimeModelConfig, ModelError> {
        if !matches!(kind, AutoRouterKind::Embedding | AutoRouterKind::Rerank) {
            return Err(ModelError::InvalidProtocolModelType);
        }
        if choice
            .model_code
            .as_ref()
            .is_some_and(|code| code.trim().is_empty())
            || choice
                .provider_id
                .as_ref()
                .is_some_and(|code| code.trim().is_empty())
        {
            return Err(ModelError::RuntimeResolutionFailed(
                "empty model or provider constraint".into(),
            ));
        }
        let requested = match choice.model_code.as_deref() {
            None => None,
            Some(code)
                if match kind {
                    AutoRouterKind::Embedding => matches!(
                        code,
                        "AUTO:Embedding"
                            | "AUTO:TextEmbedding"
                            | "AUTO:MultimodalEmbedding"
                            | "PERSON_AI:ROUTE:EmbeddingAuto"
                            | "PERSON_AI:ROUTE:MultimodalEmbeddingAuto"
                    ),
                    _ => matches!(
                        code,
                        "AUTO:Rerank"
                            | "AUTO:TextRerank"
                            | "AUTO:MultimodalRerank"
                            | "PERSON_AI:ROUTE:RerankAuto"
                    ),
                } =>
            {
                None
            }
            Some(code) if code.starts_with("AUTO:") || code.contains(":ROUTE:") => {
                return Err(ModelError::RuntimeResolutionFailed(
                    "unknown or incompatible automatic route".into(),
                ));
            }
            Some(code) => Some(code),
        };
        let catalog = self.automatic_route_catalog()?;
        let sources = self
            .automatic_route_sources()?
            .remove(kind.code())
            .unwrap_or_default();
        let eligible = catalog
            .iter()
            .filter(|model| {
                model.eligible_for(kind)
                    && requested
                        .is_none_or(|code| code == model.model_code || code == model.binding_id)
                    && choice
                        .provider_id
                        .as_ref()
                        .is_none_or(|id| id == &model.provider_id)
                    && choice
                        .required_modalities
                        .iter()
                        .all(|m| model.input_modalities.contains(m))
                    && (kind != AutoRouterKind::Embedding
                        || dimensions.is_none_or(|d| model.dimension == Some(d)))
                    && model.allowed_by(&sources)
            })
            .collect::<Vec<_>>();
        // A text-only request should not change vector spaces merely because
        // a multimodal model sorts before the text model in the catalog.
        // Explicit model/provider constraints have already filtered this pool.
        let first = eligible
            .iter()
            .min_by_key(|model| {
                model
                    .input_modalities
                    .iter()
                    .filter(|modality| !choice.required_modalities.contains(modality))
                    .count()
            })
            .ok_or_else(|| {
                ModelError::RuntimeResolutionFailed(
                    "no authorized model satisfies the route constraints".into(),
                )
            })?;
        let group_code = first.model_code.clone();
        let fixed_binding = requested == Some(first.binding_id.as_str());
        let logical = if fixed_binding {
            first.binding_id.clone()
        } else {
            group_code.clone()
        };
        let shared_identity = first.shared_identity && !fixed_binding;
        let dimension = first.dimension;
        let mut candidates = Vec::new();
        for model in eligible.into_iter().filter(|model| {
            model.model_code == group_code
                && (kind != AutoRouterKind::Embedding || model.dimension == dimension)
        }) {
            let config = if model.binding_id.starts_with("PERSON_AI:") {
                match kind {
                    AutoRouterKind::Embedding => self
                        .providers
                        .embedding_runtime_model_config(workspace, Some(&model.binding_id)),
                    _ => self
                        .providers
                        .rerank_runtime_model_config(workspace, Some(&model.binding_id)),
                }
                .map_err(provider_error)?
            } else {
                self.auxiliary_runtime_model(Some(&model.binding_id), model.model_type)?
            }
            .ok_or_else(|| {
                ModelError::RuntimeResolutionFailed("route binding unavailable".into())
            })?;
            candidates.push(RuntimeModelEndpoint {
                provider_id: model.provider_id.clone(),
                model_code: logical.clone(),
                input_modalities: model.input_modalities.clone(),
                dimension: model.dimension,
                config,
            });
        }
        let mut config = candidates[0].config.clone();
        config.model_code = logical.clone();
        config.route = Some(Box::new(RuntimeModelRoute {
            model_code: logical,
            shared_identity,
            required_modalities: choice.required_modalities.clone(),
            candidates,
        }));
        Ok(config)
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
