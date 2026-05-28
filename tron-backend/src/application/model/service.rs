use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use reqwest::Client;
use serde_json::{Value, json};
use url::Url;

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        agent::{AgentModelConfig, AgentModelProtocol},
        model::{
            Model, ModelCapability, ModelError, ModelMutationResult, ModelProtocol, ModelScopeKind,
            ModelSelectOption, ModelSourceKind, ModelType, ModelVerifyStatus, ModelView,
            ModelViewSourceKind, ModelWorkspacePathItem, PlatformDefaultModelAssignment,
            PlatformDefaultMutationResult, WorkspaceDefaultModelAssignment,
            WorkspaceDefaultModelView, WorkspaceDefaultMutationResult, WorkspaceModelCatalog,
            default_model_capabilities_for_type, validate_model_configuration,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::{id::SnowflakeId, time::current_timestamp},
};

mod catalog_resolution;
mod model_factory;
mod platform_use_cases;
mod verification;
mod workspace_mapping;
mod workspace_use_cases;

use self::{catalog_resolution::*, model_factory::*, verification::*, workspace_mapping::*};

const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MASKED_API_KEY_VALUE: &str = "*****";

#[derive(Debug, Clone)]
pub struct ModelCommand {
    pub display_name: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub model_capabilities: Vec<ModelCapability>,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key_ref: String,
    pub model_name: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub thinking_enabled: bool,
    pub structured_output_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SetAllowInheritanceCommand {
    pub allow_inheritance: bool,
}

pub trait ModelRepository: Send {
    fn list_models(&mut self) -> Result<Vec<Model>, ModelError>;
    fn next_id(&mut self) -> Result<i64, ModelError>;
    fn create_model(&mut self, model: &Model) -> Result<(), ModelError>;
    fn update_model(&mut self, model: &Model) -> Result<(), ModelError>;
    fn delete_model(&mut self, model_code: &str) -> Result<bool, ModelError>;
    fn list_workspace_default_assignments(
        &mut self,
    ) -> Result<Vec<WorkspaceDefaultModelAssignment>, ModelError>;
    fn list_platform_default_assignments(
        &mut self,
    ) -> Result<Vec<PlatformDefaultModelAssignment>, ModelError>;
    fn upsert_workspace_default_assignment(
        &mut self,
        assignment: &WorkspaceDefaultModelAssignment,
    ) -> Result<(), ModelError>;
    fn upsert_platform_default_assignment(
        &mut self,
        assignment: &PlatformDefaultModelAssignment,
    ) -> Result<(), ModelError>;
    fn delete_workspace_default_assignments_by_model_code(
        &mut self,
        model_code: &str,
    ) -> Result<(), ModelError>;
    fn delete_workspace_default_assignments_by_workspace_and_model_code(
        &mut self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<(), ModelError>;
    fn delete_platform_default_assignments_by_model_code(
        &mut self,
        model_code: &str,
    ) -> Result<(), ModelError>;
}

#[derive(Clone)]
pub struct ModelService {
    repository: Arc<Mutex<Box<dyn ModelRepository>>>,
    workspaces: WorkspaceService,
    http_client: Client,
}

#[derive(Debug, Clone)]
struct NormalizedModelSpec {
    display_name: String,
    vendor: String,
    protocol: ModelProtocol,
    model_type: ModelType,
    model_capabilities: Vec<ModelCapability>,
    base_url: String,
    api_path: Option<String>,
    api_key_ref: String,
    model_name: String,
    enabled: bool,
    description: Option<String>,
    thinking_enabled: bool,
    structured_output_enabled: bool,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    context_window_tokens: Option<u32>,
    top_p: Option<f64>,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
    allow_inheritance: Option<bool>,
}

#[derive(Clone)]
struct LineageEntry {
    code: String,
    name: String,
}

#[derive(Clone)]
struct VerifyOutcome {
    message: String,
    status: ModelVerifyStatus,
}

#[derive(Clone)]
struct EffectiveProvider {
    model: Model,
    workspace: Option<LineageEntry>,
    workspace_path: Vec<LineageEntry>,
}

#[derive(Clone)]
struct ResolvedWorkspaceRecord {
    view: ModelView,
    definition_model: Model,
    current_owned_record: Option<Model>,
    upstream_enabled: Option<bool>,
}

#[derive(Clone)]
struct ResolvedWorkspaceCatalog {
    catalog: WorkspaceModelCatalog,
    records: Vec<ResolvedWorkspaceRecord>,
}

impl ModelService {
    pub fn new(repository: impl ModelRepository + 'static, workspaces: WorkspaceService) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            http_client: Client::builder()
                .build()
                .expect("model verify client should initialize"),
        }
    }

    fn set_platform_enabled(
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
        let defaults_by_type = self.platform_default_map()?;
        Ok(build_platform_view(platform, &defaults_by_type))
    }

    fn set_workspace_enabled(
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

    fn resolve_workspace_catalog(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<WorkspaceModelCatalog, ModelError> {
        Ok(self.resolve_workspace_catalog_state(workspace)?.catalog)
    }

    fn resolve_workspace_catalog_state(
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

    fn resolve_workspace_record_state(
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

    async fn verify_model_definition(&self, model: &Model) -> Result<VerifyOutcome, ModelError> {
        let endpoint_path = model
            .api_path
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_verify_path(model.protocol, model.model_type).into());
        let url = join_verify_url(&model.base_url, &endpoint_path)?;
        let payload = verify_request_payload(model.protocol, model.model_type, &model.model_name);
        let timeout = Duration::from_millis(model.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
        let attempts = model.max_retries.unwrap_or(0).saturating_add(1);
        let api_key = self.resolve_api_key_value(model)?;
        let method_label = format!("POST {}", url.path());
        let mut last_error = None;

        for _ in 0..attempts {
            let response = self
                .http_client
                .post(url.clone())
                .timeout(timeout)
                .bearer_auth(api_key.clone())
                .json(&payload)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    return Ok(VerifyOutcome {
                        message: format!("Verified via {method_label}"),
                        status: ModelVerifyStatus::Succeeded,
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    last_error = Some(format!(
                        "{method_label} returned {status}{}",
                        if body.trim().is_empty() {
                            String::new()
                        } else {
                            format!(": {}", truncate_message(&body))
                        }
                    ));
                }
                Err(error) => {
                    last_error = Some(format!(
                        "{method_label} failed: {}",
                        truncate_message(&error.to_string())
                    ));
                }
            }
        }

        Err(ModelError::VerificationFailed(
            last_error.unwrap_or_else(|| format!("{method_label} failed")),
        ))
    }

    fn resolve_api_key_value(&self, model: &Model) -> Result<String, ModelError> {
        let reference = model.api_key_ref.trim();
        if reference.is_empty() {
            return Err(ModelError::ApiKeyRefRequired);
        }
        Ok(reference.to_string())
    }

    async fn verify_model_and_persist(&self, model: Model) -> Result<Model, ModelError> {
        match self.verify_model_definition(&model).await {
            Ok(outcome) => self.apply_verify_outcome(model, outcome),
            Err(ModelError::VerificationFailed(message)) => {
                self.apply_verify_outcome(
                    model,
                    VerifyOutcome {
                        message: message.clone(),
                        status: ModelVerifyStatus::Failed,
                    },
                )?;
                Err(ModelError::VerificationFailed(message))
            }
            Err(error) => Err(error),
        }
    }

    async fn verify_model_preview(&self, model: Model) -> Result<Model, ModelError> {
        match self.verify_model_definition(&model).await {
            Ok(outcome) => Ok(apply_verify_outcome_to_model(model, outcome)),
            Err(error) => Err(error),
        }
    }

    fn apply_verify_outcome(
        &self,
        model: Model,
        outcome: VerifyOutcome,
    ) -> Result<Model, ModelError> {
        let model = apply_verify_outcome_to_model(model, outcome);
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .update_model(&model)?;
        Ok(model)
    }

    fn all_models(&self) -> Result<Vec<Model>, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_models()
    }

    fn all_workspace_default_assignments(
        &self,
    ) -> Result<Vec<WorkspaceDefaultModelAssignment>, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_workspace_default_assignments()
    }

    fn all_platform_default_assignments(
        &self,
    ) -> Result<Vec<PlatformDefaultModelAssignment>, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_platform_default_assignments()
    }

    fn next_id(&self) -> Result<i64, ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .next_id()
    }

    fn delete_workspace_default_assignments_by_model_code(
        &self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_workspace_default_assignments_by_model_code(model_code)
    }

    fn delete_workspace_default_assignments_by_workspace_and_model_code(
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

    fn delete_platform_default_assignments_by_model_code(
        &self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_platform_default_assignments_by_model_code(model_code)
    }

    fn platform_default_map(&self) -> Result<HashMap<ModelType, String>, ModelError> {
        let models = self.all_models()?;
        let assignments = self.all_platform_default_assignments()?;
        Ok(resolve_platform_defaults(&models, &assignments))
    }

    fn find_workspace_slot_dependents(
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
