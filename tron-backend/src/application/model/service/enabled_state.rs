use super::*;

impl ModelService {
    pub(super) fn set_platform_enabled(
        &self,
        model_code: &str,
        enabled: bool,
    ) -> Result<ModelView, ModelError> {
        let models = self.all_models()?;
        let mut platform = find_platform_model(&models, model_code)?.clone();
        platform.enabled = enabled;
        platform.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .update_model(&platform)?;
        if !enabled {
            self.delete_workspace_default_assignments_by_model_code(model_code)?;
            self.delete_platform_default_assignments_by_model_code(model_code)?;
        }
        self.get_platform_model(model_code)
    }

    pub(super) fn set_workspace_enabled(
        &self,
        workspace_code: &str,
        model_code: &str,
        enabled: bool,
    ) -> Result<ModelView, ModelError> {
        let (workspace, _, record) =
            self.resolve_workspace_record_state(workspace_code, model_code)?;

        match record.view.source_kind {
            ModelViewSourceKind::Local => {
                let mut owned = record
                    .current_owned_record
                    .ok_or(ModelError::StorageFailure)?;
                owned.enabled = enabled;
                owned.updated_at = current_timestamp();
                self.repository
                    .lock()
                    .expect("model repository mutex poisoned")
                    .update_model(&owned)?;
                if !enabled {
                    self.delete_workspace_default_assignments_by_model_code(
                        &record.view.model_code,
                    )?;
                }
            }
            ModelViewSourceKind::Override => {
                let mut owned = record
                    .current_owned_record
                    .ok_or(ModelError::StorageFailure)?;
                owned.enabled = enabled;
                owned.updated_at = current_timestamp();
                self.repository
                    .lock()
                    .expect("model repository mutex poisoned")
                    .update_model(&owned)?;
                if !enabled {
                    self.delete_workspace_default_assignments_by_workspace_and_model_code(
                        &workspace.workspace_code,
                        &record.view.model_code,
                    )?;
                }
            }
            ModelViewSourceKind::Inherited => {
                let upstream_enabled = record.upstream_enabled.unwrap_or(record.view.enabled);
                match record.current_owned_record.clone() {
                    Some(existing)
                        if existing.source_kind == ModelSourceKind::WorkspaceStatusPatch =>
                    {
                        if enabled == upstream_enabled {
                            let deleted = self
                                .repository
                                .lock()
                                .expect("model repository mutex poisoned")
                                .delete_model(&existing.model_code)?;
                            if !deleted {
                                return Err(ModelError::NotFound);
                            }
                        } else {
                            let updated = update_workspace_status_patch_model(
                                existing,
                                &record.view.model_code,
                                &record.definition_model,
                                enabled,
                            );
                            self.repository
                                .lock()
                                .expect("model repository mutex poisoned")
                                .update_model(&updated)?;
                        }
                    }
                    Some(_) => return Err(ModelError::StorageFailure),
                    None => {
                        if enabled != upstream_enabled {
                            let created = workspace_status_patch_model(
                                &workspace,
                                &record.view.model_code,
                                &record.definition_model,
                                self.next_id()?,
                                enabled,
                            );
                            self.repository
                                .lock()
                                .expect("model repository mutex poisoned")
                                .create_model(&created)?;
                        }
                    }
                }
                if !enabled {
                    self.delete_workspace_default_assignments_by_workspace_and_model_code(
                        &workspace.workspace_code,
                        &record.view.model_code,
                    )?;
                }
            }
            ModelViewSourceKind::Platform => return Err(ModelError::StorageFailure),
        }

        self.get_workspace_model(&workspace.workspace_code, &record.view.model_code)
    }
}
