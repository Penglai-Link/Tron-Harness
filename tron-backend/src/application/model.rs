use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        model::{
            ModelError, PLATFORM_SOURCE_LABEL, PLATFORM_WORKSPACE_CODE, PLATFORM_WORKSPACE_NAME,
            PlatformModel, PlatformModelStatus, WorkspaceModelMutationResult, WorkspaceModelQuery,
            WorkspaceModelRecord, WorkspaceModelRemoval, WorkspaceModelStatus,
            validate_model_code,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::id::SnowflakeId,
};

const DEFAULT_TIMESTAMP: &str = "2026-05-22T00:00:00Z";

pub trait ModelRepository: Send {
    fn list_platform_models(&mut self) -> Result<Vec<PlatformModel>, ModelError>;
    fn list_workspace_model_removals(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceModelRemoval>, ModelError>;
    fn next_id(&mut self) -> Result<i64, ModelError>;
    fn create_workspace_model_removal(
        &mut self,
        removal: &WorkspaceModelRemoval,
    ) -> Result<(), ModelError>;
    fn delete_workspace_model_removal(
        &mut self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<bool, ModelError>;
}

#[derive(Clone)]
pub struct ModelService {
    repository: Arc<Mutex<Box<dyn ModelRepository>>>,
    workspaces: WorkspaceService,
}

impl ModelService {
    pub fn new(repository: impl ModelRepository + 'static, workspaces: WorkspaceService) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
        }
    }

    pub fn list_models(
        &self,
        workspace_code: &str,
        query: WorkspaceModelQuery,
    ) -> Result<Vec<WorkspaceModelRecord>, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        let search = normalized_search(query.search);
        let removals_by_model = self
            .workspace_removals_by_model(&workspace.workspace_code)?
            .into_iter()
            .map(|removal| (removal.model_code.clone(), removal))
            .collect::<BTreeMap<_, _>>();

        let mut records = self
            .enabled_platform_models()?
            .into_iter()
            .map(|model| {
                let removal = removals_by_model.get(&model.model_code);
                WorkspaceModelRecord {
                    model_code: model.model_code,
                    name: model.name,
                    purpose: model.purpose,
                    vendor: model.vendor,
                    context: model.context_window,
                    max_output: model.max_output,
                    training_data: model.training_data,
                    speed: model.speed,
                    price: model.price,
                    status: if removal.is_some() {
                        WorkspaceModelStatus::Removed
                    } else {
                        WorkspaceModelStatus::Available
                    },
                    source_workspace_code: PLATFORM_WORKSPACE_CODE.into(),
                    source_workspace_name: PLATFORM_WORKSPACE_NAME.into(),
                    source_label: PLATFORM_SOURCE_LABEL.into(),
                    removed_at: removal.map(|record| record.removed_at.clone()),
                }
            })
            .collect::<Vec<_>>();

        if let Some(status) = query.status {
            records.retain(|record| record.status == status);
        }
        if let Some(vendor) = query.vendor {
            records.retain(|record| record.vendor == vendor);
        }
        if let Some(search) = search {
            records.retain(|record| matches_search(record, &search));
        }

        records.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.model_code.cmp(&right.model_code))
        });

        Ok(records)
    }

    pub fn remove_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<WorkspaceModelMutationResult, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        if workspace.workspace_code == PLATFORM_WORKSPACE_CODE {
            return Err(ModelError::PlatformMutationDenied);
        }

        let normalized_model_code = normalized_model_code(model_code)?;
        self.ensure_enabled_platform_model(&normalized_model_code)?;

        let already_removed = self
            .workspace_removals_by_model(&workspace.workspace_code)?
            .into_iter()
            .find(|record| record.model_code == normalized_model_code);

        let removed_at = if let Some(removal) = already_removed {
            removal.removed_at
        } else {
            let id = self
                .repository
                .lock()
                .expect("model repository mutex poisoned")
                .next_id()?;
            let removal = WorkspaceModelRemoval {
                id: SnowflakeId::new(id),
                workspace_code: workspace.workspace_code.clone(),
                model_code: normalized_model_code.clone(),
                removed_at: DEFAULT_TIMESTAMP.into(),
            };
            self.repository
                .lock()
                .expect("model repository mutex poisoned")
                .create_workspace_model_removal(&removal)?;
            removal.removed_at
        };

        Ok(WorkspaceModelMutationResult {
            workspace_code: workspace.workspace_code,
            model_code: normalized_model_code,
            status: WorkspaceModelStatus::Removed,
            removed_at: Some(removed_at),
        })
    }

    pub fn restore_model(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<WorkspaceModelMutationResult, ModelError> {
        let workspace = self.active_workspace_detail(workspace_code)?;
        if workspace.workspace_code == PLATFORM_WORKSPACE_CODE {
            return Err(ModelError::PlatformMutationDenied);
        }

        let normalized_model_code = normalized_model_code(model_code)?;
        self.ensure_enabled_platform_model(&normalized_model_code)?;

        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_workspace_model_removal(&workspace.workspace_code, &normalized_model_code)?;

        Ok(WorkspaceModelMutationResult {
            workspace_code: workspace.workspace_code,
            model_code: normalized_model_code,
            status: WorkspaceModelStatus::Available,
            removed_at: None,
        })
    }

    fn active_workspace_detail(&self, workspace_code: &str) -> Result<WorkspaceDetail, ModelError> {
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

    fn enabled_platform_models(&self) -> Result<Vec<PlatformModel>, ModelError> {
        Ok(self
            .repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_platform_models()?
            .into_iter()
            .filter(|record| record.status == PlatformModelStatus::Enabled)
            .collect())
    }

    fn ensure_enabled_platform_model(&self, model_code: &str) -> Result<(), ModelError> {
        let exists = self
            .enabled_platform_models()?
            .into_iter()
            .any(|record| record.model_code == model_code);
        if exists {
            Ok(())
        } else {
            Err(ModelError::ModelNotFound)
        }
    }

    fn workspace_removals_by_model(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceModelRemoval>, ModelError> {
        if workspace_code == PLATFORM_WORKSPACE_CODE {
            return Ok(Vec::new());
        }
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_workspace_model_removals(workspace_code)
    }
}

fn map_workspace_error(error: WorkspaceError) -> ModelError {
    match error {
        WorkspaceError::StorageFailure => ModelError::StorageFailure,
        WorkspaceError::NotFound => ModelError::InvalidWorkspace,
        _ => ModelError::InvalidWorkspace,
    }
}

fn normalized_model_code(value: &str) -> Result<String, ModelError> {
    validate_model_code(value)?;
    Ok(value.trim().to_string())
}

fn normalized_search(value: Option<String>) -> Option<String> {
    value.map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

fn matches_search(record: &WorkspaceModelRecord, search: &str) -> bool {
    [
        record.model_code.as_str(),
        record.name.as_str(),
        record.purpose.as_str(),
        record.context.as_str(),
        record.price.as_str(),
        record.source_workspace_code.as_str(),
        record.source_workspace_name.as_str(),
        record.source_label.as_str(),
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}
