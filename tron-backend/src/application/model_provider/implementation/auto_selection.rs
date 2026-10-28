use super::*;
use eliteforge_agent_capability_personai::{PERSONAI_PROVIDER, PersonAiAutoRouter};
use eliteforge_agent_core as fw;

pub(super) fn default_auto_routers() -> fw::AutoRouters {
    let policy = Arc::new(PersonAiAutoRouter::default());
    fw::AutoRouters {
        chat: Some(policy.clone()),
        text_generation: Some(Arc::new(
            eliteforge_agent_capability_personai::PersonAiTextGenerationAutoRouter,
        )),
        embedding: Some(policy.clone()),
        multimodal_embedding: Some(policy.clone()),
        rerank: Some(policy),
        ..Default::default()
    }
}

impl ModelProviderService {
    pub(crate) fn text_generation_candidates(
        &self,
        workspace: &str,
    ) -> Result<Vec<fw::AutoRouteCandidate>, ModelProviderError> {
        let available = self
            .list_temporary_text_select_options(workspace)?
            .into_iter()
            .filter(|option| {
                option.enabled
                    && option.remote_status == ProviderCatalogModelStatus::Available
                    && option.selection.kind == ModelSelectionKind::Fixed
            })
            .filter_map(|option| {
                let quality = option.quality_type?;
                let code = option.selection.model_code?;
                let mut candidate = fw::AutoRouteCandidate::new(fw::ModelRef::new(
                    "personai",
                    person_ai_fixed_runtime_code(quality, &code),
                ));
                candidate.protocol = match preferred_protocol(&option.protocols) {
                    AgentModelProtocol::Responses => "openai.responses",
                    AgentModelProtocol::ChatCompletions => "openai.chat-completions",
                    AgentModelProtocol::Embeddings => "openai.embeddings",
                    AgentModelProtocol::Anthropic => "anthropic.messages",
                    AgentModelProtocol::Gemini => "gemini.generate-content",
                }
                .into();
                candidate.context_window_tokens = option.context_window_tokens.map(|n| n as usize);
                candidate
                    .metadata
                    .insert(PERSONAI_PROVIDER.into(), true.into());
                candidate.metadata.insert(
                    eliteforge_agent_capability_personai::PERSONAI_QUALITY.into(),
                    quality.storage_value().into(),
                );
                Some(candidate)
            })
            .collect::<Vec<_>>();
        Ok(eliteforge_agent_capability_personai::personai_text_generation_candidates(&available))
    }

    pub(crate) fn text_generation_auto_config(
        &self,
        workspace: &str,
    ) -> Result<RuntimeModelConfig, ModelProviderError> {
        let available = self.text_generation_candidates(workspace)?;
        if available.is_empty() {
            let english = crate::shared::auth_holder::AuthHolder::get()
                .is_some_and(|context| context.locale == "en-US");
            return Err(ModelProviderError::InvalidSelection(if english {
                "Text generation auto route has no available Middle/Low-quality PersonAI models. Sync the model catalog."
            } else {
                "文本生成自动路由没有可用的 PersonAI 中、低质量模型，请同步模型目录。"
            }.into()));
        }
        let context = fw::AutoRouteContext {
            operation: fw::ModelOperation::Chat,
            stage: "text-generation",
            affinity: workspace,
            call_key: "text-generation",
            previous: None,
            transient_failures: 0,
        };
        let model = self
            .auto_routers
            .select_text_generation(&context, &available)
            .map_err(|error| ModelProviderError::InvalidSelection(error.to_string()))?;
        let encoded = model
            .model
            .strip_prefix("PERSON_AI:FIXED:")
            .and_then(|value| value.split_once(':'))
            .ok_or_else(|| {
                ModelProviderError::InvalidSelection("invalid text generation route result".into())
            })?;
        let quality = ProviderModelQuality::from_storage(encoded.0).ok_or_else(|| {
            ModelProviderError::InvalidSelection("invalid text generation route quality".into())
        })?;
        self.resolve_temporary_text_selection(
            workspace,
            &ModelSelection::person_ai_fixed(encoded.1, quality),
        )?
        .map(runtime_config_from_agent)
        .ok_or_else(|| ModelProviderError::CatalogModelUnavailable(model.model))
    }

    pub fn with_auto_routers(mut self, routers: fw::AutoRouters) -> Self {
        self.auto_routers = routers;
        self
    }

    pub(super) fn select_auto_model(
        &self,
        operation: fw::ModelOperation,
        workspace: &str,
        available: Vec<fw::AutoRouteCandidate>,
        multimodal: bool,
    ) -> Result<fw::ModelRef, ModelProviderError> {
        let workspace = workspace.trim();
        if !workspace.is_empty() && workspace != "PLATFORM" {
            let code = workspace.to_owned();
            let active = self.connection.call(move |connection| {
                connection.query_row("SELECT EXISTS(SELECT 1 FROM workspaces WHERE workspace_code=?1 AND status='active')", [&code], |row| row.get::<_, bool>(0))
                    .map_err(|_| ModelProviderError::Storage)
            }, || ModelProviderError::Storage)?;
            if !active {
                return Err(ModelProviderError::InvalidSelection(
                    "automatic route requires an active workspace".into(),
                ));
            }
        }
        let mut routers = self.auto_routers.clone();
        if routers.source.is_none() {
            routers.source = Some(Arc::new(
                crate::application::model_router::TronAutoRouteDataSource(available.clone()),
            ));
        }
        let context = fw::AutoRouteContext {
            operation,
            stage: "model-configuration",
            affinity: workspace,
            call_key: "model-configuration",
            previous: None,
            transient_failures: 0,
        };
        (if multimodal {
            routers.select_multimodal_embedding(&context, &available)
        } else {
            routers.select(&context, &available)
        })
        .map_err(|error| ModelProviderError::InvalidSelection(error.to_string()))
    }

    pub(super) fn auto_embedding_config(
        &self,
        workspace: &str,
    ) -> Result<RuntimeModelConfig, ModelProviderError> {
        self.auto_embedding_config_kind(workspace, false)
    }
    pub(super) fn auto_multimodal_embedding_config(
        &self,
        workspace: &str,
    ) -> Result<RuntimeModelConfig, ModelProviderError> {
        self.auto_embedding_config_kind(workspace, true)
    }
    fn auto_embedding_config_kind(
        &self,
        workspace: &str,
        multimodal: bool,
    ) -> Result<RuntimeModelConfig, ModelProviderError> {
        let candidates = self.connection.call(move |connection| {
            let mut statement = connection.prepare("SELECT model_code,dimension,tags_json FROM provider_multimedia_catalog_models WHERE provider_code='PersonAi' AND media_type=6 AND status='Available' AND length(trim(model_code))>0 ORDER BY model_code").map_err(|_| ModelProviderError::Storage)?;
            let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<usize>>(1)?, row.get::<_, String>(2)?))).map_err(|_| ModelProviderError::Storage)?;
            let mut models = vec![];
            for row in rows {
                let (code, dimension, tags) = row.map_err(|_| ModelProviderError::Storage)?;
                models.push(eliteforge_agent_capability_personai::PersonAiEmbeddingModel {
                    model: fw::ModelRef::new("personai", format!("{PERSON_AI_EMBEDDING_PREFIX}{code}")),
                    dimension, tags: serde_json::from_str(&tags).unwrap_or_default(),
                });
            }
            Ok(eliteforge_agent_capability_personai::personai_embedding_candidates(models,
                if multimodal { eliteforge_agent_capability_personai::PersonAiEmbeddingKind::Multimodal } else { eliteforge_agent_capability_personai::PersonAiEmbeddingKind::Text }, Some(1024)))
        }, || ModelProviderError::Storage)?;
        if candidates.is_empty() {
            let english = crate::shared::auth_holder::AuthHolder::get()
                .is_some_and(|context| context.locale == "en-US");
            let message = match (multimodal, english) {
                (false, false) => {
                    "向量自动路由没有可用候选：Person-AI 模型列表须声明 text_embedding 标签和 1024 维向量，请检查平台标签并同步模型目录后重试索引。"
                }
                (true, false) => {
                    "多模态向量自动路由没有可用候选：Person-AI 模型列表须声明 multimodal_embedding 标签和 1024 维向量，请检查平台标签并同步模型目录后重试索引。"
                }
                (false, true) => {
                    "No text embedding route candidates: Person-AI must declare text_embedding tags and 1024 dimensions. Check platform tags, sync the model catalog, then retry indexing."
                }
                (true, true) => {
                    "No multimodal embedding route candidates: Person-AI must declare multimodal_embedding tags and 1024 dimensions. Check platform tags, sync the model catalog, then retry indexing."
                }
            };
            return Err(ModelProviderError::InvalidSelection(message.into()));
        }
        let selected = self.select_auto_model(
            fw::ModelOperation::Embedding,
            workspace,
            candidates,
            multimodal,
        )?;
        let code = selected
            .model
            .strip_prefix(PERSON_AI_EMBEDDING_PREFIX)
            .ok_or_else(|| {
                ModelProviderError::InvalidSelection("invalid embedding route result".into())
            })?;
        Ok(person_ai_embedding_config(code))
    }

    pub(super) fn auto_rerank_config(
        &self,
        workspace: &str,
    ) -> Result<RuntimeModelConfig, ModelProviderError> {
        let candidates = self.connection.call(
            |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT model_id FROM provider_multimedia_catalog_models
                 WHERE provider_code='PersonAi' AND media_type=7 AND status='Available'
                   AND length(trim(model_id)) > 0 ORDER BY model_id",
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                statement
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(|_| ModelProviderError::Storage)?
                    .map(|id| {
                        id.map(|id| {
                            let mut candidate = fw::AutoRouteCandidate::new(fw::ModelRef::new(
                                "personai",
                                format!("{PERSON_AI_RERANK_PREFIX}{id}"),
                            ));
                            candidate.protocol = "personai.rerank".into();
                            candidate
                                .metadata
                                .insert(PERSONAI_PROVIDER.into(), true.into());
                            candidate
                        })
                        .map_err(|_| ModelProviderError::Storage)
                    })
                    .collect::<Result<Vec<_>, _>>()
            },
            || ModelProviderError::Storage,
        )?;
        let selected =
            self.select_auto_model(fw::ModelOperation::Rerank, workspace, candidates, false)?;
        let id = selected
            .model
            .strip_prefix(PERSON_AI_RERANK_PREFIX)
            .ok_or_else(|| {
                ModelProviderError::InvalidSelection("invalid rerank route result".into())
            })?;
        Ok(person_ai_rerank_config(id))
    }

    /// Installed non-chat routes for every consumer of the shared model picker.
    pub fn list_auxiliary_route_options(
        &self,
        workspace: &str,
    ) -> Result<Vec<ProviderModelSelectOption>, ModelProviderError> {
        let workspace = workspace.to_owned();
        self.connection.call(
            move |connection| {
                let Some(provider) = read_connection(connection, ModelProviderCode::PersonAi)?
                else {
                    return Ok(vec![]);
                };
                if !matches!(
                    provider.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                ) {
                    return Ok(vec![]);
                }
                [
                    (ModelType::Embedding, ProviderRouteCode::EmbeddingAuto),
                    (
                        ModelType::Embedding,
                        ProviderRouteCode::MultimodalEmbeddingAuto,
                    ),
                    (ModelType::Rerank, ProviderRouteCode::RerankAuto),
                ]
                .into_iter()
                .map(|(model_type, route)| {
                    let selection = ModelSelection::person_ai_route(route);
                    Ok(ProviderModelSelectOption {
                        is_default: effective_default_selection(
                            connection, &workspace, model_type,
                        )?
                        .as_ref()
                            == Some(&selection),
                        selection,
                        option_code: format!("provider:PersonAi:route:{}", route.storage_value()),
                        display_name: route.display_name().into(),
                        provider_code: ModelProviderCode::PersonAi,
                        access_platform: ModelProviderCode::PersonAi.display_name().into(),
                        upstream_vendor_name: None,
                        model_type,
                        protocols: vec![],
                        enabled: true,
                        remote_status: ProviderCatalogModelStatus::Available,
                        context_window_tokens: None,
                        quality_type: None,
                        input_coefficient: None,
                        cache_hit_coefficient: None,
                        output_coefficient: None,
                    })
                })
                .collect()
            },
            || ModelProviderError::Storage,
        )
    }
}

pub(super) fn person_ai_embedding_config(code: &str) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: format!("{PERSON_AI_EMBEDDING_PREFIX}{code}"),
        protocol: ModelProtocol::OpenAiCompatibleEmbedding,
        model_type: ModelType::Embedding,
        base_url: "https://person-ai.invalid".into(),
        api_path: Some("embeddings".into()),
        api_key: String::new(),
        request_headers: Default::default(),
        model_name: code.into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    }
}

pub(super) fn person_ai_rerank_config(id: &str) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: format!("{PERSON_AI_RERANK_PREFIX}{id}"),
        protocol: ModelProtocol::Rerank,
        model_type: ModelType::Rerank,
        base_url: "https://person-ai.invalid".into(),
        api_path: Some("/rerank".into()),
        api_key: String::new(),
        request_headers: Default::default(),
        model_name: id.into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    }
}
