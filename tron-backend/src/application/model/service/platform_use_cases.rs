use super::*;

impl ModelService {
    pub fn list_platform_models(&self) -> Result<Vec<ModelView>, ModelError> {
        let models = self.all_models()?;
        let assignments = self.all_platform_default_assignments()?;
        let defaults_by_type = resolve_platform_defaults(&models, &assignments);
        Ok(platform_models(&models)
            .into_iter()
            .map(|model| build_platform_view(model, &defaults_by_type))
            .collect())
    }

    pub fn create_platform_model(&self, command: ModelCommand) -> Result<ModelView, ModelError> {
        let spec = normalize_model_spec(command, false, true)?;
        let models = self.all_models()?;
        ensure_platform_display_name_unique(&models, &spec.display_name, None)?;
        let assignments = self.all_platform_default_assignments()?;
        let defaults_by_type = resolve_platform_defaults(&models, &assignments);

        let id = self.next_id()?;
        let mut model = build_platform_model_from_spec(spec, id, build_model_code(id));
        model.api_key_ref = self.protect_api_key_ref(&model.api_key_ref)?;

        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .create_model(&model)?;

        if model.enabled && !defaults_by_type.contains_key(&model.model_type) {
            self.repository
                .lock()
                .expect("model repository mutex poisoned")
                .upsert_platform_default_assignment(&PlatformDefaultModelAssignment {
                    model_type: model.model_type,
                    model_code: model.model_code.clone(),
                    updated_at: current_timestamp(),
                })?;
        }

        let next_defaults_by_type = self.platform_default_map()?;
        Ok(build_platform_view(model, &next_defaults_by_type))
    }

    pub async fn verify_platform_model_create_draft(
        &self,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        let spec = normalize_model_spec(command, false, true)?;
        let models = self.all_models()?;
        ensure_platform_display_name_unique(&models, &spec.display_name, None)?;
        let assignments = self.all_platform_default_assignments()?;
        let defaults_by_type = resolve_platform_defaults(&models, &assignments);
        let preview = build_platform_model_from_spec(spec, 0, String::new());
        let verified = self.verify_model_preview(preview).await?;
        Ok(build_platform_view(verified, &defaults_by_type))
    }

    pub fn duplicate_platform_model(&self, model_code: &str) -> Result<ModelView, ModelError> {
        self.ensure_independent_model(model_code)?;
        let models = self.all_models()?;
        let source = find_platform_model(&models, model_code)?.clone();
        let id = self.next_id()?;
        let duplicated = Model {
            provider_id: None,
            request_headers: Default::default(),
            id: SnowflakeId::new(id),
            scope_kind: ModelScopeKind::Platform,
            workspace_code: None,
            model_code: build_model_code(id),
            display_name: next_platform_copy_display_name(&models, &source.display_name),
            vendor: source.vendor,
            protocol: source.protocol,
            model_type: source.model_type,
            model_capabilities: source.model_capabilities,
            base_url: source.base_url,
            api_path: source.api_path,
            api_key_ref: source.api_key_ref,
            model_name: source.model_name,
            enabled: source.enabled,
            description: source.description,
            thinking_enabled: source.thinking_enabled,
            supported_reasoning_efforts: source.supported_reasoning_efforts,
            default_reasoning_effort: source.default_reasoning_effort,
            structured_output_enabled: source.structured_output_enabled,
            responses_stateful_enabled: source.responses_stateful_enabled,
            temperature: source.temperature,
            max_tokens: source.max_tokens,
            context_window_tokens: source.context_window_tokens,
            top_p: source.top_p,
            max_retries: source.max_retries,
            last_verified_at: None,
            last_verify_status: None,
            last_verify_message: None,
            source_kind: ModelSourceKind::Platform,
            source_model_code: None,
            allow_inheritance: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };

        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .create_model(&duplicated)?;

        let defaults_by_type = self.platform_default_map()?;
        Ok(build_platform_view(duplicated, &defaults_by_type))
    }

    pub fn get_platform_model(&self, model_code: &str) -> Result<ModelView, ModelError> {
        let models = self.all_models()?;
        let model = find_platform_model(&models, model_code)?.clone();
        let assignments = self.all_platform_default_assignments()?;
        let defaults_by_type = resolve_platform_defaults(&models, &assignments);
        Ok(build_platform_view(model, &defaults_by_type))
    }

    pub fn update_platform_model(
        &self,
        model_code: &str,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        self.ensure_independent_model(model_code)?;
        let spec = normalize_model_spec(command, false, false)?;
        let models = self.all_models()?;
        ensure_platform_display_name_unique(&models, &spec.display_name, Some(model_code))?;
        let existing = find_platform_model(&models, model_code)?.clone();
        let model_type_changed = existing.model_type != spec.model_type;
        let was_enabled = existing.enabled;
        let mut updated = update_model_from_spec(existing, spec, None);
        updated.api_key_ref = self.protect_api_key_ref(&updated.api_key_ref)?;

        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .update_model(&updated)?;
        if model_type_changed {
            self.delete_workspace_default_assignments_by_model_code(model_code)?;
            self.delete_platform_default_assignments_by_model_code(model_code)?;
        }
        if was_enabled && !updated.enabled {
            self.delete_workspace_default_assignments_by_model_code(model_code)?;
            self.delete_platform_default_assignments_by_model_code(model_code)?;
        }
        let defaults_by_type = self.platform_default_map()?;
        Ok(build_platform_view(updated, &defaults_by_type))
    }

    pub async fn verify_platform_model(&self, model_code: &str) -> Result<ModelView, ModelError> {
        let model = {
            let models = self.all_models()?;
            find_platform_model(&models, model_code)?.clone()
        };
        let verified = self.verify_model_and_persist(model).await?;
        let defaults_by_type = self.platform_default_map()?;
        Ok(build_platform_view(verified, &defaults_by_type))
    }

    pub async fn verify_platform_model_draft(
        &self,
        model_code: &str,
        command: ModelCommand,
    ) -> Result<ModelView, ModelError> {
        self.ensure_independent_model(model_code)?;
        let spec = normalize_model_spec(command, false, false)?;
        let models = self.all_models()?;
        let existing = find_platform_model(&models, model_code)?.clone();
        let assignments = self.all_platform_default_assignments()?;
        let defaults_by_type = resolve_platform_defaults(&models, &assignments);
        let preview = update_model_from_spec(existing, spec, None);
        let verified = self.verify_model_preview(preview).await?;
        Ok(build_platform_view(verified, &defaults_by_type))
    }

    pub fn enable_platform_model(&self, model_code: &str) -> Result<ModelView, ModelError> {
        self.set_platform_enabled(model_code, true)
    }

    pub fn disable_platform_model(&self, model_code: &str) -> Result<ModelView, ModelError> {
        self.set_platform_enabled(model_code, false)
    }

    pub fn set_platform_default(
        &self,
        model_code: &str,
    ) -> Result<PlatformDefaultMutationResult, ModelError> {
        let models = self.all_models()?;
        let record = find_platform_model(&models, model_code)?;
        if !record.enabled {
            return Err(ModelError::DefaultTargetDisabled);
        }

        let assignment = PlatformDefaultModelAssignment {
            model_type: record.model_type,
            model_code: record.model_code.clone(),
            updated_at: current_timestamp(),
        };
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .upsert_platform_default_assignment(&assignment)?;

        Ok(PlatformDefaultMutationResult {
            model_type: assignment.model_type,
            model_code: assignment.model_code,
        })
    }

    pub fn delete_platform_model(
        &self,
        model_code: &str,
    ) -> Result<ModelMutationResult, ModelError> {
        self.ensure_independent_model(model_code)?;
        let models = self.all_models()?;
        find_platform_model(&models, model_code)?;
        let dependent_model_codes = models
            .iter()
            .filter(|model| {
                model.scope_kind == ModelScopeKind::Workspace
                    && model.source_model_code.as_deref() == Some(model_code)
            })
            .map(|model| model.model_code.clone())
            .collect::<Vec<_>>();

        {
            let mut repository = self
                .repository
                .lock()
                .expect("model repository mutex poisoned");
            let deleted = repository.delete_model(model_code)?;
            if !deleted {
                return Err(ModelError::NotFound);
            }
            for dependent_model_code in &dependent_model_codes {
                repository.delete_model(dependent_model_code)?;
            }
        }
        self.delete_workspace_default_assignments_by_model_code(model_code)?;
        self.delete_platform_default_assignments_by_model_code(model_code)?;

        Ok(ModelMutationResult {
            workspace_code: None,
            model_code: model_code.trim().to_string(),
        })
    }
}
