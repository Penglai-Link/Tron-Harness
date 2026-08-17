use super::*;

impl ModelService {
    pub fn list_workspace_models(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceModelCatalog, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        self.resolve_workspace_catalog(&workspace)
    }

    pub fn list_workspace_select_options(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<ModelSelectOption>, ModelError> {
        let catalog = self.list_workspace_models(workspace_code)?;
        Ok(catalog
            .records
            .into_iter()
            .map(|record| ModelSelectOption {
                model_code: record.model_code,
                display_name: record.display_name,
                model_name: record.model_name,
                source_kind: record.source_kind,
                source_label: record.source_label,
                vendor: record.vendor,
                protocol: record.protocol,
                model_type: record.model_type,
                enabled: record.enabled,
            })
            .collect())
    }

    pub fn default_text_agent_model_config(
        &self,
        workspace_code: &str,
    ) -> Result<Option<AgentModelConfig>, ModelError> {
        self.resolve_text_agent_model_config(workspace_code, None, None)
    }

    pub fn text_agent_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<AgentModelConfig>, ModelError> {
        self.resolve_text_agent_model_config(workspace_code, model_code, None)
    }

    pub fn text_agent_model_config_with_reasoning_effort(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
        reasoning_effort: Option<ReasoningEffort>,
    ) -> Result<Option<AgentModelConfig>, ModelError> {
        self.resolve_text_agent_model_config(workspace_code, model_code, reasoning_effort)
    }

    pub fn text_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        self.resolve_runtime_model_config(workspace_code, model_code, ModelType::Text)
    }

    pub fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        self.resolve_runtime_model_config(workspace_code, model_code, ModelType::Embedding)
    }

    pub fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        self.resolve_runtime_model_config(workspace_code, model_code, ModelType::Rerank)
    }

    fn resolve_text_agent_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
        reasoning_effort: Option<ReasoningEffort>,
    ) -> Result<Option<AgentModelConfig>, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        let requested_model_code = model_code.map(str::trim).filter(|value| !value.is_empty());
        let resolved_model_code = if let Some(model_code) = requested_model_code {
            model_code
        } else {
            let Some(default_model_code) = catalog_state
                .catalog
                .defaults
                .iter()
                .find(|default| default.model_type == ModelType::Text)
                .and_then(|default| default.model_code.as_deref())
            else {
                return Ok(None);
            };
            default_model_code
        };
        let Some(record) = catalog_state.records.iter().find(|record| {
            record.view.model_code == resolved_model_code
                && record.view.enabled
                && record.view.model_type == ModelType::Text
        }) else {
            return Ok(None);
        };

        if reasoning_effort
            .is_some_and(|effort| !record.view.supported_reasoning_efforts.contains(&effort))
        {
            return Err(ModelError::ReasoningEffortUnsupported);
        }
        let mut config =
            agent_model_config_from_view(&record.view, &record.definition_model.api_key_ref);
        if let (Some(config), Some(reasoning_effort)) = (&mut config, reasoning_effort) {
            config.reasoning_effort = Some(reasoning_effort);
        }
        Ok(config)
    }

    fn resolve_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
        model_type: ModelType,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        let requested_model_code = model_code.map(str::trim).filter(|value| !value.is_empty());
        let resolved_model_code = if let Some(model_code) = requested_model_code {
            model_code
        } else {
            let Some(default_model_code) = catalog_state
                .catalog
                .defaults
                .iter()
                .find(|default| default.model_type == model_type)
                .and_then(|default| default.model_code.as_deref())
            else {
                return Ok(None);
            };
            default_model_code
        };
        let Some(record) = catalog_state.records.iter().find(|record| {
            record.view.model_code == resolved_model_code
                && record.view.enabled
                && record.view.model_type == model_type
        }) else {
            return Ok(None);
        };

        Ok(runtime_model_config_from_view(
            &record.view,
            &record.definition_model.api_key_ref,
        ))
    }

    pub fn create_workspace_model(
        &self,
        workspace_code: &str,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let spec = normalize_model_spec(command, true, true)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        ensure_workspace_catalog_display_name_unique(
            &catalog_state.records,
            &workspace.workspace_code,
            &spec.display_name,
            None,
        )?;

        let id = self.next_id()?;
        let model = build_workspace_local_model_from_spec(
            &workspace.workspace_code,
            spec,
            id,
            build_model_code(id),
        );

        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .create_model(&model)?;

        self.get_workspace_model(&workspace.workspace_code, &model.model_code)
    }

    pub async fn verify_workspace_model_create_draft(
        &self,
        workspace_code: &str,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let spec = normalize_model_spec(command, true, true)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        ensure_workspace_catalog_display_name_unique(
            &catalog_state.records,
            &workspace.workspace_code,
            &spec.display_name,
            None,
        )?;
        let preview = build_workspace_local_model_from_spec(
            &workspace.workspace_code,
            spec,
            0,
            String::new(),
        );
        let verified = self.verify_model_preview(preview).await?;
        Ok(build_workspace_effective_view(
            &workspace,
            verified.model_code.as_str(),
            &verified,
            ModelViewSourceKind::Local,
            None,
            verified.enabled,
            verified.allow_inheritance,
            &verified,
        ))
    }

    pub fn duplicate_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<ModelView, ModelError> {
        let (workspace, catalog_state, resolved) =
            self.resolve_workspace_record_state(workspace_code, model_code)?;

        let id = self.next_id()?;
        let duplicated = duplicate_workspace_source_model(
            &resolved.definition_model,
            &workspace,
            id,
            next_workspace_copy_display_name(
                &catalog_state.catalog.records,
                &resolved.view.display_name,
            ),
        );
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .create_model(&duplicated)?;

        self.get_workspace_model(&workspace.workspace_code, &duplicated.model_code)
    }

    pub fn get_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<ModelView, ModelError> {
        let (_, _, record) = self.resolve_workspace_record_state(workspace_code, model_code)?;
        Ok(record.view)
    }

    pub fn update_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let spec = normalize_model_spec(command, true, false)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        let resolved = catalog_state
            .records
            .iter()
            .find(|record| record.view.model_code == model_code.trim())
            .cloned()
            .ok_or(ModelError::NotFound)?;

        ensure_workspace_catalog_display_name_unique(
            &catalog_state.records,
            &workspace.workspace_code,
            &spec.display_name,
            Some(model_code),
        )?;

        match resolved.view.source_kind {
            ModelViewSourceKind::Local => {
                let existing = resolved
                    .current_owned_record
                    .clone()
                    .ok_or(ModelError::StorageFailure)?;
                let model_type_changed = existing.model_type != spec.model_type;
                let was_enabled = existing.enabled;
                let allow_inheritance = existing.allow_inheritance;
                let updated = update_model_from_spec(existing, spec.clone(), allow_inheritance);

                self.repository
                    .lock()
                    .expect("model repository mutex poisoned")
                    .update_model(&updated)?;
                if model_type_changed || (was_enabled && !updated.enabled) {
                    self.delete_workspace_default_assignments_by_model_code(&updated.model_code)?;
                }
            }
            ModelViewSourceKind::Inherited | ModelViewSourceKind::Override => {
                match resolved.current_owned_record.clone() {
                    Some(existing)
                        if existing.source_kind == ModelSourceKind::WorkspaceOverride
                            || existing.source_kind == ModelSourceKind::WorkspaceStatusPatch =>
                    {
                        let model_type_changed = existing.model_type != spec.model_type;
                        let was_enabled = existing.enabled;
                        let updated = update_workspace_override_model(
                            existing,
                            &resolved.view.model_code,
                            spec.clone(),
                            &resolved.definition_model.api_key_ref,
                        );
                        self.repository
                            .lock()
                            .expect("model repository mutex poisoned")
                            .update_model(&updated)?;
                        if model_type_changed || (was_enabled && !updated.enabled) {
                            self.delete_workspace_default_assignments_by_workspace_and_model_code(
                                &workspace.workspace_code,
                                &resolved.view.model_code,
                            )?;
                        }
                        updated
                    }
                    Some(_) => return Err(ModelError::StorageFailure),
                    None => {
                        let created = workspace_override_model(
                            &workspace,
                            &resolved.view.model_code,
                            self.next_id()?,
                            spec.clone(),
                            &resolved.definition_model.api_key_ref,
                        );
                        self.repository
                            .lock()
                            .expect("model repository mutex poisoned")
                            .create_model(&created)?;
                        created
                    }
                };
                return self
                    .get_workspace_model(&workspace.workspace_code, &resolved.view.model_code);
            }
            ModelViewSourceKind::Platform => return Err(ModelError::StorageFailure),
        }

        self.get_workspace_model(&workspace.workspace_code, model_code)
    }

    pub async fn verify_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<ModelView, ModelError> {
        let (workspace, _, record) =
            self.resolve_workspace_record_state(workspace_code, model_code)?;
        self.verify_model_and_persist(record.definition_model.clone())
            .await?;
        self.get_workspace_model(&workspace.workspace_code, &record.view.model_code)
    }

    pub async fn verify_workspace_model_draft(
        &self,
        workspace_code: &str,
        model_code: &str,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let spec = normalize_model_spec(command, true, false)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        let resolved = catalog_state
            .records
            .iter()
            .find(|record| record.view.model_code == model_code.trim())
            .cloned()
            .ok_or(ModelError::NotFound)?;

        let preview_definition = match resolved.view.source_kind {
            ModelViewSourceKind::Local => {
                let existing = resolved
                    .current_owned_record
                    .clone()
                    .ok_or(ModelError::StorageFailure)?;
                let allow_inheritance = existing.allow_inheritance;
                update_model_from_spec(existing, spec, allow_inheritance)
            }
            ModelViewSourceKind::Inherited | ModelViewSourceKind::Override => {
                let fallback_api_key_ref = resolved.definition_model.api_key_ref.clone();
                match resolved.current_owned_record.clone() {
                    Some(existing)
                        if existing.source_kind == ModelSourceKind::WorkspaceOverride
                            || existing.source_kind == ModelSourceKind::WorkspaceStatusPatch =>
                    {
                        update_workspace_override_model(
                            existing,
                            &resolved.view.model_code,
                            spec,
                            &fallback_api_key_ref,
                        )
                    }
                    Some(_) => return Err(ModelError::StorageFailure),
                    None => {
                        let mut preview = resolved.definition_model.clone();
                        preview.scope_kind = ModelScopeKind::Workspace;
                        preview.workspace_code = Some(workspace.workspace_code.clone());
                        preview.source_kind = ModelSourceKind::WorkspaceOverride;
                        preview.source_model_code = Some(resolved.view.model_code.clone());
                        preview.allow_inheritance = None;
                        update_workspace_override_model(
                            preview,
                            &resolved.view.model_code,
                            spec,
                            &fallback_api_key_ref,
                        )
                    }
                }
            }
            ModelViewSourceKind::Platform => return Err(ModelError::StorageFailure),
        };

        let verified = self.verify_model_preview(preview_definition).await?;
        Ok(build_workspace_preview_view(
            &resolved.view,
            &verified,
            resolved.view.enabled,
            match resolved.view.source_kind {
                ModelViewSourceKind::Local => resolved.view.allow_inheritance,
                _ => None,
            },
        ))
    }

    pub fn enable_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<ModelView, ModelError> {
        self.set_workspace_enabled(workspace_code, model_code, true)
    }

    pub fn disable_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<ModelView, ModelError> {
        self.set_workspace_enabled(workspace_code, model_code, false)
    }

    pub fn set_workspace_default(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<WorkspaceDefaultMutationResult, ModelError> {
        let (workspace, _, record) =
            self.resolve_workspace_record_state(workspace_code, model_code)?;
        if !record.view.enabled {
            return Err(ModelError::DefaultTargetDisabled);
        }

        let assignment = WorkspaceDefaultModelAssignment {
            workspace_code: workspace.workspace_code.clone(),
            model_type: record.view.model_type,
            model_code: record.view.model_code.clone(),
            updated_at: current_timestamp(),
        };
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .upsert_workspace_default_assignment(&assignment)?;

        Ok(WorkspaceDefaultMutationResult {
            workspace_code: assignment.workspace_code,
            model_type: assignment.model_type,
            model_code: assignment.model_code,
        })
    }

    pub fn set_workspace_allow_inheritance(
        &self,
        workspace_code: &str,
        model_code: &str,
        command: SetAllowInheritanceCommand,
    ) -> Result<ModelView, ModelError> {
        let (workspace, _, record) =
            self.resolve_workspace_record_state(workspace_code, model_code)?;
        if record.view.source_kind != ModelViewSourceKind::Local {
            return Err(ModelError::AllowInheritanceUnsupported);
        }
        let mut owned = record
            .current_owned_record
            .ok_or(ModelError::StorageFailure)?;

        owned.allow_inheritance = Some(command.allow_inheritance);
        owned.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .update_model(&owned)?;
        self.get_workspace_model(&workspace.workspace_code, &owned.model_code)
    }

    pub fn delete_workspace_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<ModelMutationResult, ModelError> {
        let (workspace, _, record) =
            self.resolve_workspace_record_state(workspace_code, model_code)?;
        match record.view.source_kind {
            ModelViewSourceKind::Local => {
                let owned = record
                    .current_owned_record
                    .ok_or(ModelError::StorageFailure)?;
                let dependent_model_codes =
                    self.find_workspace_slot_dependents(&owned.model_code)?;
                {
                    let mut repository = self
                        .repository
                        .lock()
                        .expect("model repository mutex poisoned");
                    let deleted = repository.delete_model(&owned.model_code)?;
                    if !deleted {
                        return Err(ModelError::NotFound);
                    }
                    for dependent_model_code in &dependent_model_codes {
                        repository.delete_model(dependent_model_code)?;
                    }
                }
                self.delete_workspace_default_assignments_by_model_code(&record.view.model_code)?;
                Ok(ModelMutationResult {
                    workspace_code: Some(workspace.workspace_code),
                    model_code: record.view.model_code,
                })
            }
            ModelViewSourceKind::Override => {
                let owned = record
                    .current_owned_record
                    .ok_or(ModelError::StorageFailure)?;
                let deleted = self
                    .repository
                    .lock()
                    .expect("model repository mutex poisoned")
                    .delete_model(&owned.model_code)?;
                if !deleted {
                    return Err(ModelError::NotFound);
                }
                Ok(ModelMutationResult {
                    workspace_code: Some(workspace.workspace_code),
                    model_code: record.view.model_code,
                })
            }
            ModelViewSourceKind::Inherited => Err(ModelError::InheritedMutationDenied),
            ModelViewSourceKind::Platform => Err(ModelError::StorageFailure),
        }
    }
}
