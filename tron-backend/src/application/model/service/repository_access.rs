use super::*;

impl ModelService {
    pub(super) fn all_models(&self) -> Result<Vec<Model>, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_models()
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
