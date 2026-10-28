use super::*;

impl ModelService {
    pub(super) fn all_models(&self) -> Result<Vec<Model>, ModelError> {
        let mut repository = self
            .repository
            .lock()
            .expect("model repository mutex poisoned");
        let (mut models, providers) = repository.model_configuration_snapshot()?;
        let root_enabled: HashMap<_, _> = models
            .iter()
            .map(|model| (model.model_code.clone(), model.enabled))
            .collect();
        let bindings: HashMap<_, _> = providers
            .iter()
            .flat_map(|provider| {
                provider.models.iter().filter_map(move |model| {
                    model.model_code.as_deref().map(|code| (code, provider))
                })
            })
            .collect();
        for model in &mut models {
            let connection = bindings.get(model.model_code.as_str()).or_else(|| {
                model
                    .source_model_code
                    .as_deref()
                    .and_then(|code| bindings.get(code))
            });
            if let Some(provider) = connection {
                model.provider_id = Some(provider.provider_id.clone());
                model.request_headers = provider.request_headers.clone();
                model.base_url = provider.base_url.clone();
                model.api_path = None;
                model.api_key_ref = provider.encrypted_api_key.clone();
                model.protocol = provider.protocol.model_protocol();
                model.vendor = provider.display_name.clone();
                model.enabled &= provider.enabled;
                if let Some(source) = &model.source_model_code {
                    model.enabled &= root_enabled.get(source).copied().unwrap_or(false);
                }
            }
        }
        Ok(models)
    }

    pub(super) fn all_workspace_default_assignments(
        &self,
    ) -> Result<Vec<WorkspaceDefaultModelAssignment>, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_workspace_default_assignments()
    }

    pub(super) fn all_platform_default_assignments(
        &self,
    ) -> Result<Vec<PlatformDefaultModelAssignment>, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_platform_default_assignments()
    }

    pub(super) fn next_id(&self) -> Result<i64, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .next_id()
    }

    pub(super) fn delete_workspace_default_assignments_by_model_code(
        &self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_workspace_default_assignments_by_model_code(model_code)
    }

    pub(super) fn delete_workspace_default_assignments_by_workspace_and_model_code(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_workspace_default_assignments_by_workspace_and_model_code(
                workspace_code,
                model_code,
            )
    }

    pub(super) fn delete_platform_default_assignments_by_model_code(
        &self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_platform_default_assignments_by_model_code(model_code)
    }

    pub(super) fn platform_default_map(&self) -> Result<HashMap<ModelType, String>, ModelError> {
        let models = self.all_models()?;
        let assignments = self.all_platform_default_assignments()?;
        Ok(resolve_platform_defaults(&models, &assignments))
    }

    pub(super) fn find_workspace_slot_dependents(
        &self,
        slot_model_code: &str,
    ) -> Result<Vec<String>, ModelError> {
        let normalized = slot_model_code.trim();
        Ok(self
            .all_models()?
            .into_iter()
            .filter(|model| {
                model.scope_kind == ModelScopeKind::Workspace
                    && model.source_model_code.as_deref() == Some(normalized)
            })
            .map(|model| model.model_code)
            .collect())
    }
}
