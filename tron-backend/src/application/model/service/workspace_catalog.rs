use super::*;

impl ModelService {
    pub(super) fn active_workspace_detail(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceDetail, ModelError> {
        let code = workspace_code.trim();
        if code.is_empty() {
            return Err(ModelError::WorkspaceRequired);
        }

        let workspace = self
            .workspaces
            .get_workspace(code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(ModelError::InvalidWorkspace);
        }
        Ok(workspace)
    }

    pub(super) fn resolve_workspace_catalog(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<WorkspaceModelCatalog, ModelError> {
        Ok(self.resolve_workspace_catalog_state(workspace)?.catalog)
    }

    pub(super) fn resolve_workspace_catalog_state(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<ResolvedWorkspaceCatalog, ModelError> {
        let models = self.all_models()?;
        let workspace_assignments = self.all_workspace_default_assignments()?;
        let platform_assignments = self.all_platform_default_assignments()?;
        let lineage = workspace_path_entries(workspace);
        let lineage_map: HashMap<_, _> = lineage
            .iter()
            .map(|entry| (entry.code.clone(), entry.clone()))
            .collect();
        let workspace_slot_records: HashMap<_, _> = models
            .iter()
            .filter(|model| {
                model.scope_kind == ModelScopeKind::Workspace
                    && matches!(
                        model.source_kind,
                        ModelSourceKind::WorkspaceOverride | ModelSourceKind::WorkspaceStatusPatch
                    )
            })
            .filter_map(|model| {
                Some((
                    (
                        model.workspace_code.clone()?,
                        model.source_model_code.clone()?,
                    ),
                    model.clone(),
                ))
            })
            .collect();

        let current_workspace_code = workspace.workspace_code.as_str();
        let mut roots = platform_models(&models);
        roots.extend(
            models
                .iter()
                .filter(|model| {
                    if model.scope_kind != ModelScopeKind::Workspace
                        || model.source_kind != ModelSourceKind::WorkspaceLocal
                        || model.source_model_code.is_some()
                    {
                        return false;
                    }
                    let Some(owner_workspace_code) = model.workspace_code.as_deref() else {
                        return false;
                    };
                    owner_workspace_code == current_workspace_code
                        || (lineage_map.contains_key(owner_workspace_code)
                            && model.allow_inheritance.unwrap_or(true))
                })
                .cloned(),
        );

        let mut records = roots
            .iter()
            .map(|root| {
                build_resolved_workspace_record(
                    root,
                    workspace,
                    &lineage,
                    &lineage_map,
                    &workspace_slot_records,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        sort_resolved_workspace_records(&mut records);

        let defaults_by_type = resolve_effective_workspace_defaults(
            &records,
            &workspace_assignments,
            &platform_assignments,
            workspace,
            &models,
        );
        for record in &mut records {
            record.view.is_default = defaults_by_type
                .get(&record.view.model_type)
                .is_some_and(|value| *value == record.view.model_code.as_str());
        }

        let catalog = WorkspaceModelCatalog {
            workspace_code: workspace.workspace_code.clone(),
            defaults: model_type_sequence()
                .into_iter()
                .map(|model_type| {
                    let matching = defaults_by_type.get(&model_type).and_then(|model_code| {
                        records
                            .iter()
                            .find(|record| record.view.model_code == *model_code)
                    });
                    WorkspaceDefaultModelView {
                        model_type,
                        model_code: matching.map(|record| record.view.model_code.clone()),
                        display_name: matching.map(|record| record.view.display_name.clone()),
                        source_label: matching.map(|record| record.view.source_label.clone()),
                    }
                })
                .collect(),
            records: records.iter().map(|record| record.view.clone()).collect(),
        };

        Ok(ResolvedWorkspaceCatalog { catalog, records })
    }

    pub(super) fn resolve_workspace_record_state(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<
        (
            WorkspaceDetail,
            ResolvedWorkspaceCatalog,
            ResolvedWorkspaceRecord,
        ),
        ModelError,
    > {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let catalog_state = self.resolve_workspace_catalog_state(&workspace)?;
        let record = catalog_state
            .records
            .iter()
            .find(|record| record.view.model_code == model_code.trim())
            .cloned()
            .ok_or(ModelError::NotFound)?;
        Ok((workspace, catalog_state, record))
    }
}
