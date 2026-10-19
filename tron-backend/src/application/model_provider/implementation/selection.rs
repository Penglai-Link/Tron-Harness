use super::*;

impl ModelProviderService {
    /// Resolve the Auto catalog in one database operation, not one lookup per model.
    pub fn auto_chat_candidates(
        &self,
        allowed: &[String],
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    ) -> Result<Vec<(String, ModelSelection, AgentModelConfig)>, ModelProviderError> {
        let allowed = allowed.to_vec();
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
                Ok(
                    read_chat_catalog_models(connection, ModelProviderCode::PersonAi)?
                        .into_iter()
                        .filter(|m| {
                            m.status == ProviderCatalogModelStatus::Available
                                && m.quality_type == Some(ProviderModelQuality::High)
                                && m.context_window_tokens.is_some_and(|n| n > 0)
                        })
                        .filter_map(|model| {
                            let quality = model.quality_type?;
                            let code = model.selection_model_code();
                            let option = format!(
                                "provider:PersonAi:model:{}:{}",
                                quality.storage_value(),
                                code
                            );
                            if !allowed.is_empty() && !allowed.contains(&option) {
                                return None;
                            }
                            let config = person_ai_agent_config(
                                person_ai_fixed_runtime_code(quality, &code),
                                &code,
                                model.context_window_tokens,
                                reasoning_effort,
                                preferred_protocol(&model.protocols),
                            );
                            Some((
                                option,
                                ModelSelection::person_ai_fixed(code, quality),
                                config,
                            ))
                        })
                        .collect(),
                )
            },
            || ModelProviderError::Storage,
        )
    }

    pub fn list_text_select_options(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<ProviderModelSelectOption>, ModelProviderError> {
        self.text_select_options(workspace_code, true)
    }

    pub(crate) fn list_temporary_text_select_options(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<ProviderModelSelectOption>, ModelProviderError> {
        self.text_select_options(workspace_code, false)
    }

    fn text_select_options(
        &self,
        workspace_code: &str,
        high_quality_only: bool,
    ) -> Result<Vec<ProviderModelSelectOption>, ModelProviderError> {
        let this = self.clone();
        let worker_workspace_code = workspace_code.to_owned();
        this.connection.clone().call(
            move |connection| {
                let workspace_code = &worker_workspace_code;
                let mut default_selection =
                    effective_default_selection(&connection, workspace_code, ModelType::Text)?;
                if let Some(selection) = default_selection.as_mut()
                    && selection.provider_code == Some(ModelProviderCode::PersonAi)
                    && selection.kind == ModelSelectionKind::Fixed
                    && let Some(code) = selection.model_code.as_deref()
                {
                    let model = match read_catalog_model(
                        &connection,
                        ModelProviderCode::PersonAi,
                        code,
                        selection.quality_type,
                    ) {
                        Ok(model) => model,
                        // A stale ambiguous default must not hide the explicit vendor
                        // choices needed to replace it. Database failures still propagate.
                        Err(ModelProviderError::InvalidSelection(_)) => None,
                        Err(error) => return Err(error),
                    };
                    if let Some(model) = model {
                        selection.model_code = Some(model.selection_model_code());
                        selection.quality_type = selection.quality_type.or(model.quality_type);
                    }
                }
                let Some(provider) = read_connection(&connection, ModelProviderCode::PersonAi)?
                else {
                    return Ok(Vec::new());
                };
                let active = matches!(
                    provider.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                );
                let mut options = Vec::new();
                if active {
                    let selection = ModelSelection::person_ai_route(ProviderRouteCode::ChatAuto);
                    options.push(ProviderModelSelectOption {
                        is_default: default_selection.as_ref() == Some(&selection),
                        selection,
                        option_code: "provider:PersonAi:route:ChatAuto".into(),
                        display_name: eliteforge_agent_core::CHAT_AUTO_ROUTE_NAME.into(),
                        provider_code: ModelProviderCode::PersonAi,
                        access_platform: ModelProviderCode::PersonAi.display_name().into(),
                        upstream_vendor_name: None,
                        model_type: ModelType::Text,
                        protocols: vec![AgentModelProtocol::ChatCompletions],
                        enabled: true,
                        remote_status: ProviderCatalogModelStatus::Available,
                        context_window_tokens: None,
                        quality_type: Some(ProviderModelQuality::High),
                        input_coefficient: None,
                        cache_hit_coefficient: None,
                        output_coefficient: None,
                    });
                }
                for model in read_chat_catalog_models(&connection, ModelProviderCode::PersonAi)? {
                    if high_quality_only && model.quality_type != Some(ProviderModelQuality::High) {
                        continue;
                    }
                    let enabled = active && model.status == ProviderCatalogModelStatus::Available;
                    let selection = ModelSelection::person_ai_fixed(
                        model.selection_model_code(),
                        model
                            .quality_type
                            .expect("chat catalog models always have a quality"),
                    );
                    options.push(ProviderModelSelectOption {
                        is_default: default_selection.as_ref() == Some(&selection),
                        selection,
                        option_code: format!(
                            "provider:PersonAi:model:{}:{}",
                            model
                                .quality_type
                                .expect("chat catalog models always have a quality")
                                .storage_value(),
                            model.selection_model_code(),
                        ),
                        display_name: model.model_name.clone(),
                        provider_code: ModelProviderCode::PersonAi,
                        access_platform: ModelProviderCode::PersonAi.display_name().into(),
                        upstream_vendor_name: model.upstream_vendor_name.clone(),
                        model_type: ModelType::Text,
                        protocols: model.protocols.clone(),
                        enabled,
                        remote_status: model.status,
                        context_window_tokens: model.context_window_tokens,
                        quality_type: model.quality_type,
                        input_coefficient: model.input_coefficient,
                        cache_hit_coefficient: model.cache_hit_coefficient,
                        output_coefficient: model.output_coefficient,
                    });
                }
                Ok(options)
            },
            || ModelProviderError::Storage,
        )
    }

    pub fn resolve_chat_selection(
        &self,
        workspace_code: &str,
        selection: Option<&ModelSelection>,
        legacy_model_code: Option<&str>,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    ) -> Result<Option<AgentModelConfig>, ModelProviderError> {
        self.resolve_text_selection(
            workspace_code,
            selection,
            legacy_model_code,
            reasoning_effort,
            true,
        )
    }

    pub(crate) fn resolve_temporary_text_selection(
        &self,
        workspace_code: &str,
        selection: &ModelSelection,
    ) -> Result<Option<AgentModelConfig>, ModelProviderError> {
        self.resolve_text_selection(workspace_code, Some(selection), None, None, false)
    }

    fn resolve_text_selection(
        &self,
        workspace_code: &str,
        selection: Option<&ModelSelection>,
        legacy_model_code: Option<&str>,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
        high_quality_only: bool,
    ) -> Result<Option<AgentModelConfig>, ModelProviderError> {
        let this = self.clone();
        let worker_workspace_code = workspace_code.to_owned();
        let worker_selection = selection.map(ToOwned::to_owned);
        let worker_legacy_model_code = legacy_model_code.map(ToOwned::to_owned);
        let worker_reasoning_effort = reasoning_effort.clone();
        this.connection.clone().call(
            move |connection| {
                let workspace_code = &worker_workspace_code;
                let selection = worker_selection.as_ref();
                let legacy_model_code = worker_legacy_model_code.as_deref();
                let reasoning_effort = worker_reasoning_effort;
                let selection = match selection {
                    Some(selection) => Some(selection.clone()),
                    None => legacy_model_code
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .and_then(|model_code| {
                            read_catalog_model(
                                &connection,
                                ModelProviderCode::PersonAi,
                                model_code,
                                None,
                            )
                            .ok()
                            .flatten()
                            .map(|model| {
                                ModelSelection::person_ai_fixed(
                                    model_code,
                                    model
                                        .quality_type
                                        .expect("chat catalog models always have a quality"),
                                )
                            })
                        })
                        .or_else(|| {
                            effective_default_selection(
                                &connection,
                                workspace_code,
                                ModelType::Text,
                            )
                            .ok()
                            .flatten()
                        }),
                };
                let Some(selection) = selection else {
                    return Ok(None);
                };
                if selection.provider_code != Some(ModelProviderCode::PersonAi) {
                    return Ok(None);
                }
                let provider = read_connection(&connection, ModelProviderCode::PersonAi)?
                    .ok_or(ModelProviderError::ConnectionNotReady)?;
                if !matches!(
                    provider.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                ) {
                    return Err(
                        if provider.status == ProviderConnectionStatus::AuthRequired {
                            ModelProviderError::AuthenticationRequired
                        } else {
                            ModelProviderError::ConnectionNotReady
                        },
                    );
                }
                match selection.kind {
                    ModelSelectionKind::Auto => Err(ModelProviderError::InvalidSelection(
                        "Tron Auto must be resolved by the chat session router".into(),
                    )),
                    ModelSelectionKind::ProviderRoute => {
                        if selection.route_code != Some(ProviderRouteCode::ChatAuto)
                            || selection.model_code.is_some()
                            || selection.quality_type.is_some()
                        {
                            return Err(ModelProviderError::InvalidSelection(
                                "Person-AI chat route must be ChatAuto without modelCode".into(),
                            ));
                        }
                        Ok(Some(person_ai_agent_config(
                            "PERSON_AI:ROUTE:ChatAuto",
                            "",
                            None,
                            reasoning_effort,
                            AgentModelProtocol::ChatCompletions,
                        )))
                    }
                    ModelSelectionKind::Fixed => {
                        let model_code = selection
                            .model_code
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .ok_or_else(|| {
                                ModelProviderError::InvalidSelection(
                                    "Person-AI Fixed selection requires modelCode".into(),
                                )
                            })?;
                        let model = read_catalog_model(
                            &connection,
                            ModelProviderCode::PersonAi,
                            model_code,
                            selection.quality_type,
                        )?
                        .ok_or_else(|| {
                            ModelProviderError::CatalogModelUnavailable(model_code.into())
                        })?;
                        if high_quality_only {
                            require_high_chat_quality(model.quality_type)?;
                        }
                        if model.status != ProviderCatalogModelStatus::Available {
                            return Err(ModelProviderError::CatalogModelUnavailable(
                                model_code.into(),
                            ));
                        }
                        let canonical_code = model.selection_model_code();
                        Ok(Some(person_ai_agent_config(
                            person_ai_fixed_runtime_code(
                                model
                                    .quality_type
                                    .expect("chat catalog models always have a quality"),
                                &canonical_code,
                            ),
                            &canonical_code,
                            model.context_window_tokens,
                            reasoning_effort,
                            preferred_protocol(&model.protocols),
                        )))
                    }
                }
            },
            || ModelProviderError::Storage,
        )
    }

    pub fn text_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelProviderError> {
        let selection = match model_code.map(str::trim).filter(|value| !value.is_empty()) {
            Some(model_code) => self.person_ai_selection_from_legacy_code(model_code)?,
            None => None,
        };
        let selection = selection.or_else(|| {
            self.default_selection(workspace_code, ModelType::Text)
                .ok()
                .flatten()
        });
        let Some(selection) = selection else {
            return Ok(None);
        };
        let config = self.resolve_chat_selection(workspace_code, Some(&selection), None, None)?;
        Ok(config.map(runtime_config_from_agent))
    }

    pub fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelProviderError> {
        let requested = model_code.map(str::trim).filter(|value| !value.is_empty());
        if let Some(logical_code) =
            requested.and_then(|value| value.strip_prefix(PERSON_AI_EMBEDDING_PREFIX))
        {
            if logical_code.is_empty() {
                return Ok(None);
            }
            self.require_active_connection(ModelProviderCode::PersonAi)?;
            let code = logical_code.to_owned();
            let available = self.connection.call(
                {
                    let code = code.clone();
                    move |connection| {
                        connection
                            .query_row(
                                "SELECT EXISTS(SELECT 1 FROM provider_multimedia_catalog_models
                         WHERE provider_code = 'PersonAi' AND media_type = 6
                           AND status = 'Available' AND model_code = ?1)",
                                [&code],
                                |row| row.get::<_, bool>(0),
                            )
                            .map_err(|_| ModelProviderError::Storage)
                    }
                },
                || ModelProviderError::Storage,
            )?;
            if !available {
                return Ok(None);
            }
            return Ok(Some(super::auto_selection::person_ai_embedding_config(
                &code,
            )));
        }
        let default = if requested.is_none() {
            self.default_selection(workspace_code, ModelType::Embedding)?
        } else {
            None
        };
        if requested == Some("PERSON_AI:ROUTE:MultimodalEmbeddingAuto")
            || default.as_ref().is_some_and(|selection| {
                selection.provider_code == Some(ModelProviderCode::PersonAi)
                    && selection.kind == ModelSelectionKind::ProviderRoute
                    && selection.route_code == Some(ProviderRouteCode::MultimodalEmbeddingAuto)
            })
        {
            self.require_active_connection(ModelProviderCode::PersonAi)?;
            return self
                .auto_multimodal_embedding_config(workspace_code)
                .map(Some);
        }
        if requested.is_some_and(|value| value != "PERSON_AI:ROUTE:EmbeddingAuto") {
            return Ok(None);
        }
        let selection = default;
        if requested.is_none()
            && !selection.is_some_and(|selection| {
                selection.provider_code == Some(ModelProviderCode::PersonAi)
                    && selection.kind == ModelSelectionKind::ProviderRoute
                    && selection.route_code == Some(ProviderRouteCode::EmbeddingAuto)
            })
        {
            return Ok(None);
        }
        self.require_active_connection(ModelProviderCode::PersonAi)?;
        self.auto_embedding_config(workspace_code).map(Some)
    }

    pub fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelProviderError> {
        let requested = model_code.map(str::trim).filter(|code| !code.is_empty());
        if let Some(id) = requested.and_then(|code| code.strip_prefix(PERSON_AI_RERANK_PREFIX)) {
            self.require_active_connection(ModelProviderCode::PersonAi)?;
            let id = id.to_owned();
            let available = self.connection.call({
                let id = id.clone();
                move |connection| connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM provider_multimedia_catalog_models WHERE provider_code='PersonAi' AND media_type=7 AND status='Available' AND model_id=?1)",
                    [&id], |row| row.get::<_, bool>(0),
                ).map_err(|_| ModelProviderError::Storage)
            }, || ModelProviderError::Storage)?;
            return Ok(available.then(|| super::auto_selection::person_ai_rerank_config(&id)));
        }
        if requested.is_some_and(|code| code != "PERSON_AI:ROUTE:RerankAuto") {
            return Ok(None);
        }
        let selection = self.default_selection(workspace_code, ModelType::Rerank)?;
        if requested.is_none()
            && !selection.is_some_and(|selection| {
                selection.provider_code == Some(ModelProviderCode::PersonAi)
                    && selection.kind == ModelSelectionKind::ProviderRoute
                    && selection.route_code == Some(ProviderRouteCode::RerankAuto)
            })
        {
            return Ok(None);
        }
        self.require_active_connection(ModelProviderCode::PersonAi)?;
        self.auto_rerank_config(workspace_code).map(Some)
    }

    pub fn is_person_ai_config(&self, model_code: &str) -> bool {
        model_code.starts_with(PROVIDER_RUNTIME_PREFIX)
    }
}
