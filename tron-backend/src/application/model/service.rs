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
        let model = build_platform_model_from_spec(spec, id, build_model_code(id));

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
        let models = self.all_models()?;
        let source = find_platform_model(&models, model_code)?.clone();
        let id = self.next_id()?;
        let duplicated = Model {
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
            structured_output_enabled: source.structured_output_enabled,
            temperature: source.temperature,
            max_tokens: source.max_tokens,
            top_p: source.top_p,
            timeout_ms: source.timeout_ms,
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
        let spec = normalize_model_spec(command, false, false)?;
        let models = self.all_models()?;
        ensure_platform_display_name_unique(&models, &spec.display_name, Some(model_code))?;
        let existing = find_platform_model(&models, model_code)?.clone();
        let model_type_changed = existing.model_type != spec.model_type;
        let was_enabled = existing.enabled;
        let updated = update_model_from_spec(existing, spec, None);

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

fn platform_models(models: &[Model]) -> Vec<Model> {
    let mut records = models
        .iter()
        .filter(|model| model.scope_kind == ModelScopeKind::Platform)
        .cloned()
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.model_code.cmp(&right.model_code))
    });
    records
}

fn build_platform_view(model: Model, defaults_by_type: &HashMap<ModelType, String>) -> ModelView {
    let model_code = model.model_code.clone();
    let model_type = model.model_type;
    let is_default = defaults_by_type
        .get(&model_type)
        .is_some_and(|value| value == &model_code);
    ModelView {
        scope_kind: ModelScopeKind::Platform,
        workspace_code: None,
        model_code,
        display_name: model.display_name,
        vendor: model.vendor,
        protocol: model.protocol,
        model_type,
        model_capabilities: model.model_capabilities,
        base_url: model.base_url,
        api_path: model.api_path,
        api_key_ref: masked_api_key_ref(&model.api_key_ref),
        model_name: model.model_name,
        enabled: model.enabled,
        description: model.description,
        thinking_enabled: model.thinking_enabled,
        structured_output_enabled: model.structured_output_enabled,
        temperature: model.temperature,
        max_tokens: model.max_tokens,
        top_p: model.top_p,
        timeout_ms: model.timeout_ms,
        max_retries: model.max_retries,
        last_verified_at: model.last_verified_at,
        last_verify_status: model.last_verify_status,
        last_verify_message: model.last_verify_message,
        source_kind: ModelViewSourceKind::Platform,
        source_model_code: None,
        source_label: "Platform".into(),
        source_workspace_code: None,
        source_workspace_name: None,
        source_workspace_path: Vec::new(),
        allow_inheritance: None,
        is_default,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn build_workspace_source_label(source_kind: ModelViewSourceKind) -> &'static str {
    match source_kind {
        ModelViewSourceKind::Platform => "Platform",
        ModelViewSourceKind::Local => "Local",
        ModelViewSourceKind::Inherited => "Inherited",
        ModelViewSourceKind::Override => "Override",
    }
}

fn build_workspace_effective_view(
    workspace: &WorkspaceDetail,
    visible_model_code: &str,
    config_model: &Model,
    source_kind: ModelViewSourceKind,
    inherited_from: Option<&EffectiveProvider>,
    enabled: bool,
    allow_inheritance: Option<bool>,
    timestamps_model: &Model,
) -> ModelView {
    let (source_workspace_code, source_workspace_name) = if let Some(workspace_provider) =
        inherited_from.and_then(|provider| provider.workspace.as_ref())
    {
        (
            Some(workspace_provider.code.clone()),
            Some(workspace_provider.name.clone()),
        )
    } else {
        (None, None)
    };
    let source_workspace_path = inherited_from
        .map(|provider| {
            provider
                .workspace_path
                .iter()
                .map(|entry| ModelWorkspacePathItem {
                    workspace_code: entry.code.clone(),
                    name: entry.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    ModelView {
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: visible_model_code.to_string(),
        display_name: config_model.display_name.clone(),
        vendor: config_model.vendor.clone(),
        protocol: config_model.protocol,
        model_type: config_model.model_type,
        model_capabilities: config_model.model_capabilities.clone(),
        base_url: config_model.base_url.clone(),
        api_path: config_model.api_path.clone(),
        api_key_ref: masked_api_key_ref(&config_model.api_key_ref),
        model_name: config_model.model_name.clone(),
        enabled,
        description: config_model.description.clone(),
        thinking_enabled: config_model.thinking_enabled,
        structured_output_enabled: config_model.structured_output_enabled,
        temperature: config_model.temperature,
        max_tokens: config_model.max_tokens,
        top_p: config_model.top_p,
        timeout_ms: config_model.timeout_ms,
        max_retries: config_model.max_retries,
        last_verified_at: config_model.last_verified_at.clone(),
        last_verify_status: config_model.last_verify_status,
        last_verify_message: config_model.last_verify_message.clone(),
        source_kind,
        source_model_code: match source_kind {
            ModelViewSourceKind::Local => None,
            _ => Some(visible_model_code.to_string()),
        },
        source_label: build_workspace_source_label(source_kind).into(),
        source_workspace_code,
        source_workspace_name,
        source_workspace_path,
        allow_inheritance,
        is_default: false,
        created_at: timestamps_model.created_at.clone(),
        updated_at: timestamps_model.updated_at.clone(),
    }
}

fn build_workspace_preview_view(
    current_view: &ModelView,
    preview_model: &Model,
    enabled: bool,
    allow_inheritance: Option<bool>,
) -> ModelView {
    let mut view = current_view.clone();
    view.display_name = preview_model.display_name.clone();
    view.vendor = preview_model.vendor.clone();
    view.protocol = preview_model.protocol;
    view.model_type = preview_model.model_type;
    view.model_capabilities = preview_model.model_capabilities.clone();
    view.base_url = preview_model.base_url.clone();
    view.api_path = preview_model.api_path.clone();
    view.api_key_ref = masked_api_key_ref(&preview_model.api_key_ref);
    view.model_name = preview_model.model_name.clone();
    view.enabled = enabled;
    view.description = preview_model.description.clone();
    view.thinking_enabled = preview_model.thinking_enabled;
    view.structured_output_enabled = preview_model.structured_output_enabled;
    view.temperature = preview_model.temperature;
    view.max_tokens = preview_model.max_tokens;
    view.top_p = preview_model.top_p;
    view.timeout_ms = preview_model.timeout_ms;
    view.max_retries = preview_model.max_retries;
    view.last_verified_at = preview_model.last_verified_at.clone();
    view.last_verify_status = preview_model.last_verify_status;
    view.last_verify_message = preview_model.last_verify_message.clone();
    view.allow_inheritance = allow_inheritance;
    view.updated_at = preview_model.updated_at.clone();
    view
}

fn workspace_provider(
    model: &Model,
    workspace: &LineageEntry,
    lineage: &[LineageEntry],
) -> EffectiveProvider {
    EffectiveProvider {
        model: model.clone(),
        workspace: Some(workspace.clone()),
        workspace_path: workspace_path_to_workspace(lineage, workspace.code.as_str()),
    }
}

fn platform_provider(model: &Model) -> EffectiveProvider {
    EffectiveProvider {
        model: model.clone(),
        workspace: None,
        workspace_path: Vec::new(),
    }
}

fn workspace_path_to_workspace(
    lineage: &[LineageEntry],
    workspace_code: &str,
) -> Vec<LineageEntry> {
    let Some(index) = lineage
        .iter()
        .position(|entry| entry.code == workspace_code)
    else {
        return Vec::new();
    };
    lineage.iter().take(index + 1).cloned().collect()
}

fn find_workspace_slot_record(
    models_by_slot: &HashMap<(String, String), Model>,
    workspace_code: &str,
    slot_model_code: &str,
) -> Option<Model> {
    models_by_slot
        .get(&(workspace_code.to_string(), slot_model_code.to_string()))
        .cloned()
}

fn build_resolved_workspace_record(
    root: &Model,
    workspace: &WorkspaceDetail,
    lineage: &[LineageEntry],
    lineage_map: &HashMap<String, LineageEntry>,
    workspace_slot_records: &HashMap<(String, String), Model>,
) -> Result<ResolvedWorkspaceRecord, ModelError> {
    let current_workspace_code = workspace.workspace_code.as_str();
    let visible_model_code = root.model_code.clone();

    if root.scope_kind == ModelScopeKind::Workspace
        && root.workspace_code.as_deref() == Some(current_workspace_code)
    {
        let view = build_workspace_effective_view(
            workspace,
            &visible_model_code,
            root,
            ModelViewSourceKind::Local,
            None,
            root.enabled,
            root.allow_inheritance,
            root,
        );
        return Ok(ResolvedWorkspaceRecord {
            view,
            definition_model: root.clone(),
            current_owned_record: Some(root.clone()),
            upstream_enabled: None,
        });
    }

    let mut definition_provider = if root.scope_kind == ModelScopeKind::Platform {
        platform_provider(root)
    } else {
        let owner_workspace = root
            .workspace_code
            .as_deref()
            .and_then(|code| lineage_map.get(code))
            .ok_or(ModelError::StorageFailure)?;
        workspace_provider(root, owner_workspace, lineage)
    };
    let mut inherited_provider = definition_provider.clone();

    let ancestor_entries: Vec<&LineageEntry> = if root.scope_kind == ModelScopeKind::Platform {
        lineage
            .iter()
            .filter(|entry| entry.code != current_workspace_code)
            .collect()
    } else {
        let owner_workspace_code = root
            .workspace_code
            .as_deref()
            .ok_or(ModelError::StorageFailure)?;
        let start_index = lineage
            .iter()
            .position(|entry| entry.code == owner_workspace_code)
            .ok_or(ModelError::StorageFailure)?;
        lineage
            .iter()
            .skip(start_index + 1)
            .filter(|entry| entry.code != current_workspace_code)
            .collect()
    };

    for ancestor in ancestor_entries {
        let Some(candidate) =
            find_workspace_slot_record(workspace_slot_records, &ancestor.code, &visible_model_code)
        else {
            continue;
        };
        match candidate.source_kind {
            ModelSourceKind::WorkspaceOverride => {
                definition_provider = workspace_provider(&candidate, ancestor, lineage);
                inherited_provider = definition_provider.clone();
            }
            ModelSourceKind::WorkspaceStatusPatch => {
                inherited_provider = workspace_provider(&candidate, ancestor, lineage);
            }
            _ => {}
        }
    }

    let current_owned_record = find_workspace_slot_record(
        workspace_slot_records,
        current_workspace_code,
        &visible_model_code,
    );

    let record = match current_owned_record
        .as_ref()
        .map(|record| record.source_kind)
    {
        Some(ModelSourceKind::WorkspaceOverride) => {
            let current_override = current_owned_record
                .as_ref()
                .expect("override record should exist");
            let view = build_workspace_effective_view(
                workspace,
                &visible_model_code,
                current_override,
                ModelViewSourceKind::Override,
                Some(&inherited_provider),
                current_override.enabled,
                None,
                current_override,
            );
            ResolvedWorkspaceRecord {
                view,
                definition_model: current_override.clone(),
                current_owned_record,
                upstream_enabled: Some(inherited_provider.model.enabled),
            }
        }
        Some(ModelSourceKind::WorkspaceStatusPatch) => {
            let current_patch = current_owned_record
                .as_ref()
                .expect("status patch record should exist");
            let view = build_workspace_effective_view(
                workspace,
                &visible_model_code,
                &definition_provider.model,
                ModelViewSourceKind::Inherited,
                Some(&inherited_provider),
                current_patch.enabled,
                None,
                current_patch,
            );
            ResolvedWorkspaceRecord {
                view,
                definition_model: definition_provider.model.clone(),
                current_owned_record,
                upstream_enabled: Some(inherited_provider.model.enabled),
            }
        }
        Some(_) => return Err(ModelError::StorageFailure),
        None => {
            let view = build_workspace_effective_view(
                workspace,
                &visible_model_code,
                &definition_provider.model,
                ModelViewSourceKind::Inherited,
                Some(&inherited_provider),
                inherited_provider.model.enabled,
                None,
                &inherited_provider.model,
            );
            ResolvedWorkspaceRecord {
                view,
                definition_model: definition_provider.model.clone(),
                current_owned_record: None,
                upstream_enabled: Some(inherited_provider.model.enabled),
            }
        }
    };

    Ok(record)
}

fn sort_resolved_workspace_records(records: &mut [ResolvedWorkspaceRecord]) {
    records.sort_by(|left, right| {
        left.view
            .model_type
            .cmp(&right.view.model_type)
            .then_with(|| {
                left.view
                    .display_name
                    .to_lowercase()
                    .cmp(&right.view.display_name.to_lowercase())
            })
            .then_with(|| {
                left.view
                    .source_label
                    .to_lowercase()
                    .cmp(&right.view.source_label.to_lowercase())
            })
            .then_with(|| left.view.model_code.cmp(&right.view.model_code))
    });
}

fn resolve_effective_workspace_defaults(
    records: &[ResolvedWorkspaceRecord],
    workspace_assignments: &[WorkspaceDefaultModelAssignment],
    platform_assignments: &[PlatformDefaultModelAssignment],
    workspace: &WorkspaceDetail,
    models: &[Model],
) -> HashMap<ModelType, String> {
    let record_map: HashMap<_, _> = records
        .iter()
        .map(|record| (record.view.model_code.as_str(), &record.view))
        .collect();
    let mut workspace_resolution_order = vec![workspace.workspace_code.clone()];
    workspace_resolution_order.extend(
        workspace
            .path
            .iter()
            .rev()
            .filter(|entry| entry.workspace_code != workspace.workspace_code)
            .map(|entry| entry.workspace_code.clone()),
    );
    let platform_defaults = resolve_platform_defaults(models, platform_assignments);

    model_type_sequence()
        .into_iter()
        .filter_map(|model_type| {
            for workspace_code in &workspace_resolution_order {
                let Some(assignment) = workspace_assignments.iter().find(|assignment| {
                    assignment.workspace_code == *workspace_code
                        && assignment.model_type == model_type
                }) else {
                    continue;
                };
                let Some(record) = record_map.get(assignment.model_code.as_str()) else {
                    continue;
                };
                if record.enabled && record.model_type == model_type {
                    return Some((model_type, record.model_code.clone()));
                }
            }

            platform_defaults.get(&model_type).and_then(|model_code| {
                let record = record_map.get(model_code.as_str())?;
                if !record.enabled || record.model_type != model_type {
                    return None;
                }
                Some((model_type, record.model_code.clone()))
            })
        })
        .collect()
}

fn resolve_platform_defaults(
    models: &[Model],
    assignments: &[PlatformDefaultModelAssignment],
) -> HashMap<ModelType, String> {
    let record_map: HashMap<_, _> = models
        .iter()
        .filter(|model| model.scope_kind == ModelScopeKind::Platform)
        .map(|model| (model.model_code.as_str(), model))
        .collect();
    let explicit_defaults = assignments
        .iter()
        .filter_map(|assignment| {
            let model = record_map.get(assignment.model_code.as_str())?;
            if !model.enabled || model.model_type != assignment.model_type {
                return None;
            }
            Some((assignment.model_type, model.model_code.clone()))
        })
        .collect::<HashMap<_, _>>();

    let fallback_defaults = model_type_sequence()
        .into_iter()
        .filter_map(|model_type| {
            first_enabled_platform_model_for_type(models, model_type)
                .map(|model| (model_type, model.model_code.clone()))
        })
        .collect::<HashMap<_, _>>();

    model_type_sequence()
        .into_iter()
        .filter_map(|model_type| {
            explicit_defaults
                .get(&model_type)
                .or_else(|| fallback_defaults.get(&model_type))
                .map(|model_code| (model_type, model_code.clone()))
        })
        .collect()
}

fn first_enabled_platform_model_for_type(
    models: &[Model],
    model_type: ModelType,
) -> Option<&Model> {
    models
        .iter()
        .filter(|model| {
            model.scope_kind == ModelScopeKind::Platform
                && model.enabled
                && model.model_type == model_type
        })
        .min_by_key(|model| model.id.value())
}

fn model_type_sequence() -> [ModelType; 4] {
    [
        ModelType::Text,
        ModelType::Embedding,
        ModelType::Rerank,
        ModelType::Image,
    ]
}

fn normalize_model_spec(
    command: ModelCommand,
    workspace_local: bool,
    require_api_key: bool,
) -> Result<NormalizedModelSpec, ModelError> {
    let model_capabilities =
        normalize_model_capabilities(command.model_type, command.model_capabilities);
    let api_key_for_validation = if require_api_key || !command.api_key_ref.trim().is_empty() {
        command.api_key_ref.as_str()
    } else {
        MASKED_API_KEY_VALUE
    };
    validate_model_configuration(
        &command.display_name,
        &command.vendor,
        &command.base_url,
        api_key_for_validation,
        &command.model_name,
        command.protocol,
        command.model_type,
        &model_capabilities,
    )?;

    Ok(NormalizedModelSpec {
        display_name: command.display_name.trim().to_string(),
        vendor: command.vendor.trim().to_string(),
        protocol: command.protocol,
        model_type: command.model_type,
        model_capabilities,
        base_url: command.base_url.trim().trim_end_matches('/').to_string(),
        api_path: trim_optional(command.api_path),
        api_key_ref: command.api_key_ref.trim().to_string(),
        model_name: command.model_name.trim().to_string(),
        enabled: command.enabled,
        description: trim_optional(command.description),
        thinking_enabled: command.thinking_enabled,
        structured_output_enabled: command.structured_output_enabled,
        temperature: command.temperature,
        max_tokens: command.max_tokens,
        top_p: command.top_p,
        timeout_ms: command.timeout_ms,
        max_retries: command.max_retries,
        allow_inheritance: workspace_local.then_some(command.allow_inheritance.unwrap_or(true)),
    })
}

fn update_model_from_spec(
    mut model: Model,
    spec: NormalizedModelSpec,
    allow_inheritance: Option<bool>,
) -> Model {
    model.display_name = spec.display_name;
    model.vendor = spec.vendor;
    model.protocol = spec.protocol;
    model.model_type = spec.model_type;
    model.model_capabilities = spec.model_capabilities;
    model.base_url = spec.base_url;
    model.api_path = spec.api_path;
    if !spec.api_key_ref.is_empty() && spec.api_key_ref != MASKED_API_KEY_VALUE {
        model.api_key_ref = spec.api_key_ref;
    }
    model.model_name = spec.model_name;
    model.enabled = spec.enabled;
    model.description = spec.description;
    model.thinking_enabled = spec.thinking_enabled;
    model.structured_output_enabled = spec.structured_output_enabled;
    model.temperature = spec.temperature;
    model.max_tokens = spec.max_tokens;
    model.top_p = spec.top_p;
    model.timeout_ms = spec.timeout_ms;
    model.max_retries = spec.max_retries;
    model.allow_inheritance = allow_inheritance;
    model.updated_at = current_timestamp();
    model
}

fn normalize_model_capabilities(
    model_type: ModelType,
    values: Vec<ModelCapability>,
) -> Vec<ModelCapability> {
    let seeded = if values.is_empty() {
        default_model_capabilities_for_type(model_type)
    } else {
        values
    };
    let mut normalized = Vec::new();
    for value in seeded {
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    if normalized.is_empty() {
        return default_model_capabilities_for_type(model_type);
    }
    normalized
}

fn ensure_platform_display_name_unique(
    models: &[Model],
    display_name: &str,
    exclude_model_code: Option<&str>,
) -> Result<(), ModelError> {
    if models.iter().any(|model| {
        model.scope_kind == ModelScopeKind::Platform
            && model.display_name == display_name
            && Some(model.model_code.as_str()) != exclude_model_code
    }) {
        return Err(ModelError::DuplicatePlatformDisplayName);
    }
    Ok(())
}

fn next_platform_copy_display_name(models: &[Model], display_name: &str) -> String {
    let base_name = format!("{display_name}-Copy");
    if !models.iter().any(|model| {
        model.scope_kind == ModelScopeKind::Platform && model.display_name == base_name
    }) {
        return base_name;
    }

    let mut copy_index = 2;
    loop {
        let candidate = format!("{display_name}-Copy-{copy_index}");
        if !models.iter().any(|model| {
            model.scope_kind == ModelScopeKind::Platform && model.display_name == candidate
        }) {
            return candidate;
        }
        copy_index += 1;
    }
}

fn next_workspace_copy_display_name(records: &[ModelView], display_name: &str) -> String {
    let base_name = format!("{display_name}-Copy");
    if !records
        .iter()
        .any(|record| record.display_name == base_name)
    {
        return base_name;
    }

    let mut copy_index = 2;
    loop {
        let candidate = format!("{display_name}-Copy-{copy_index}");
        if !records
            .iter()
            .any(|record| record.display_name == candidate)
        {
            return candidate;
        }
        copy_index += 1;
    }
}

fn ensure_workspace_catalog_display_name_unique(
    records: &[ResolvedWorkspaceRecord],
    workspace_code: &str,
    display_name: &str,
    exclude_model_code: Option<&str>,
) -> Result<(), ModelError> {
    if records.iter().any(|record| {
        record.view.display_name == display_name
            && Some(record.view.model_code.as_str()) != exclude_model_code
    }) {
        return Err(ModelError::DuplicateWorkspaceDisplayName {
            workspace_code: workspace_code.into(),
        });
    }
    Ok(())
}

fn find_platform_model<'a>(models: &'a [Model], model_code: &str) -> Result<&'a Model, ModelError> {
    models
        .iter()
        .find(|model| {
            model.scope_kind == ModelScopeKind::Platform && model.model_code == model_code.trim()
        })
        .ok_or(ModelError::NotFound)
}

fn build_model_code(id: i64) -> String {
    format!("MODEL-{id}")
}

fn duplicate_workspace_source_model(
    source: &Model,
    workspace: &WorkspaceDetail,
    id: i64,
    display_name: String,
) -> Model {
    Model {
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: build_model_code(id),
        display_name,
        vendor: source.vendor.clone(),
        protocol: source.protocol,
        model_type: source.model_type,
        model_capabilities: source.model_capabilities.clone(),
        base_url: source.base_url.clone(),
        api_path: source.api_path.clone(),
        api_key_ref: source.api_key_ref.clone(),
        model_name: source.model_name.clone(),
        enabled: source.enabled,
        description: source.description.clone(),
        thinking_enabled: source.thinking_enabled,
        structured_output_enabled: source.structured_output_enabled,
        temperature: source.temperature,
        max_tokens: source.max_tokens,
        top_p: source.top_p,
        timeout_ms: source.timeout_ms,
        max_retries: source.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceLocal,
        source_model_code: None,
        allow_inheritance: Some(source.allow_inheritance.unwrap_or(true)),
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

fn status_patch_display_name(slot_model_code: &str) -> String {
    format!("__STATUS_PATCH__{slot_model_code}")
}

fn workspace_status_patch_model(
    workspace: &WorkspaceDetail,
    source_slot_code: &str,
    source: &Model,
    id: i64,
    enabled: bool,
) -> Model {
    Model {
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: build_model_code(id),
        display_name: status_patch_display_name(source_slot_code),
        vendor: source.vendor.clone(),
        protocol: source.protocol,
        model_type: source.model_type,
        model_capabilities: source.model_capabilities.clone(),
        base_url: source.base_url.clone(),
        api_path: source.api_path.clone(),
        api_key_ref: source.api_key_ref.clone(),
        model_name: source.model_name.clone(),
        enabled,
        description: source.description.clone(),
        thinking_enabled: source.thinking_enabled,
        structured_output_enabled: source.structured_output_enabled,
        temperature: source.temperature,
        max_tokens: source.max_tokens,
        top_p: source.top_p,
        timeout_ms: source.timeout_ms,
        max_retries: source.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceStatusPatch,
        source_model_code: Some(source_slot_code.to_string()),
        allow_inheritance: None,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

fn update_workspace_status_patch_model(
    mut patch: Model,
    source_slot_code: &str,
    source: &Model,
    enabled: bool,
) -> Model {
    patch.display_name = status_patch_display_name(source_slot_code);
    patch.vendor = source.vendor.clone();
    patch.protocol = source.protocol;
    patch.model_type = source.model_type;
    patch.model_capabilities = source.model_capabilities.clone();
    patch.base_url = source.base_url.clone();
    patch.api_path = source.api_path.clone();
    patch.api_key_ref = source.api_key_ref.clone();
    patch.model_name = source.model_name.clone();
    patch.enabled = enabled;
    patch.description = source.description.clone();
    patch.thinking_enabled = source.thinking_enabled;
    patch.structured_output_enabled = source.structured_output_enabled;
    patch.temperature = source.temperature;
    patch.max_tokens = source.max_tokens;
    patch.top_p = source.top_p;
    patch.timeout_ms = source.timeout_ms;
    patch.max_retries = source.max_retries;
    patch.last_verified_at = None;
    patch.last_verify_status = None;
    patch.last_verify_message = None;
    patch.source_kind = ModelSourceKind::WorkspaceStatusPatch;
    patch.source_model_code = Some(source_slot_code.to_string());
    patch.allow_inheritance = None;
    patch.updated_at = current_timestamp();
    patch
}

fn workspace_override_model(
    workspace: &WorkspaceDetail,
    source_slot_code: &str,
    id: i64,
    spec: NormalizedModelSpec,
    fallback_api_key_ref: &str,
) -> Model {
    Model {
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: build_model_code(id),
        display_name: spec.display_name,
        vendor: spec.vendor,
        protocol: spec.protocol,
        model_type: spec.model_type,
        model_capabilities: spec.model_capabilities,
        base_url: spec.base_url,
        api_path: spec.api_path,
        api_key_ref: if !spec.api_key_ref.is_empty() && spec.api_key_ref != MASKED_API_KEY_VALUE {
            spec.api_key_ref
        } else {
            fallback_api_key_ref.to_string()
        },
        model_name: spec.model_name,
        enabled: spec.enabled,
        description: spec.description,
        thinking_enabled: spec.thinking_enabled,
        structured_output_enabled: spec.structured_output_enabled,
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        top_p: spec.top_p,
        timeout_ms: spec.timeout_ms,
        max_retries: spec.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceOverride,
        source_model_code: Some(source_slot_code.to_string()),
        allow_inheritance: None,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

fn update_workspace_override_model(
    mut existing: Model,
    source_slot_code: &str,
    spec: NormalizedModelSpec,
    fallback_api_key_ref: &str,
) -> Model {
    existing.display_name = spec.display_name;
    existing.vendor = spec.vendor;
    existing.protocol = spec.protocol;
    existing.model_type = spec.model_type;
    existing.model_capabilities = spec.model_capabilities;
    existing.base_url = spec.base_url;
    existing.api_path = spec.api_path;
    existing.api_key_ref =
        if !spec.api_key_ref.is_empty() && spec.api_key_ref != MASKED_API_KEY_VALUE {
            spec.api_key_ref
        } else {
            fallback_api_key_ref.to_string()
        };
    existing.model_name = spec.model_name;
    existing.enabled = spec.enabled;
    existing.description = spec.description;
    existing.thinking_enabled = spec.thinking_enabled;
    existing.structured_output_enabled = spec.structured_output_enabled;
    existing.temperature = spec.temperature;
    existing.max_tokens = spec.max_tokens;
    existing.top_p = spec.top_p;
    existing.timeout_ms = spec.timeout_ms;
    existing.max_retries = spec.max_retries;
    existing.last_verified_at = None;
    existing.last_verify_status = None;
    existing.last_verify_message = None;
    existing.source_kind = ModelSourceKind::WorkspaceOverride;
    existing.source_model_code = Some(source_slot_code.to_string());
    existing.allow_inheritance = None;
    existing.updated_at = current_timestamp();
    existing
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn masked_api_key_ref(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        MASKED_API_KEY_VALUE.to_string()
    }
}

fn apply_verify_outcome_to_model(mut model: Model, outcome: VerifyOutcome) -> Model {
    model.last_verified_at = Some(current_timestamp());
    model.last_verify_status = Some(outcome.status);
    model.last_verify_message = Some(outcome.message);
    model.updated_at = current_timestamp();
    model
}

fn build_platform_model_from_spec(spec: NormalizedModelSpec, id: i64, model_code: String) -> Model {
    Model {
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Platform,
        workspace_code: None,
        model_code,
        display_name: spec.display_name,
        vendor: spec.vendor,
        protocol: spec.protocol,
        model_type: spec.model_type,
        model_capabilities: spec.model_capabilities,
        base_url: spec.base_url,
        api_path: spec.api_path,
        api_key_ref: spec.api_key_ref,
        model_name: spec.model_name,
        enabled: spec.enabled,
        description: spec.description,
        thinking_enabled: spec.thinking_enabled,
        structured_output_enabled: spec.structured_output_enabled,
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        top_p: spec.top_p,
        timeout_ms: spec.timeout_ms,
        max_retries: spec.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::Platform,
        source_model_code: None,
        allow_inheritance: None,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

fn build_workspace_local_model_from_spec(
    workspace_code: &str,
    spec: NormalizedModelSpec,
    id: i64,
    model_code: String,
) -> Model {
    Model {
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace_code.to_string()),
        model_code,
        display_name: spec.display_name,
        vendor: spec.vendor,
        protocol: spec.protocol,
        model_type: spec.model_type,
        model_capabilities: spec.model_capabilities,
        base_url: spec.base_url,
        api_path: spec.api_path,
        api_key_ref: spec.api_key_ref,
        model_name: spec.model_name,
        enabled: spec.enabled,
        description: spec.description,
        thinking_enabled: spec.thinking_enabled,
        structured_output_enabled: spec.structured_output_enabled,
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        top_p: spec.top_p,
        timeout_ms: spec.timeout_ms,
        max_retries: spec.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceLocal,
        source_model_code: None,
        allow_inheritance: Some(spec.allow_inheritance.unwrap_or(true)),
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

fn default_verify_path(protocol: ModelProtocol, model_type: ModelType) -> &'static str {
    match (protocol, model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => "/chat/completions",
        (ModelProtocol::OpenAiCompatible, ModelType::Embedding) => "/embeddings",
        (ModelProtocol::Responses, ModelType::Text) => "/responses",
        (ModelProtocol::Responses, ModelType::Image) => "/responses",
        (ModelProtocol::Rerank, ModelType::Rerank) => "/rerank",
        _ => "/verify",
    }
}

fn verify_request_payload(
    protocol: ModelProtocol,
    model_type: ModelType,
    model_name: &str,
) -> Value {
    match (protocol, model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => json!({
            "model": model_name,
            "messages": [{ "role": "user", "content": "ping" }],
            "max_tokens": 1
        }),
        (ModelProtocol::OpenAiCompatible, ModelType::Embedding) => json!({
            "model": model_name,
            "input": "ping"
        }),
        (ModelProtocol::Responses, ModelType::Text) => json!({
            "model": model_name,
            "input": "ping",
            "max_output_tokens": 1
        }),
        (ModelProtocol::Responses, ModelType::Image) => json!({
            "model": model_name,
            "input": [
                {
                    "role": "user",
                    "content": [
                        { "type": "input_text", "text": "Generate a verification image." }
                    ]
                }
            ],
            "max_output_tokens": 1
        }),
        (ModelProtocol::Rerank, ModelType::Rerank) => json!({
            "model": model_name,
            "query": "ping",
            "documents": ["ping"]
        }),
        _ => json!({}),
    }
}

fn join_verify_url(base_url: &str, path: &str) -> Result<Url, ModelError> {
    let mut base = Url::parse(base_url).map_err(|_| ModelError::InvalidBaseUrl)?;
    let trimmed_path = path.trim().trim_start_matches('/');
    if trimmed_path.is_empty() {
        return Ok(base);
    }

    let base_path = base.path().trim_end_matches('/');
    let joined_path = if base_path.is_empty() {
        format!("/{trimmed_path}")
    } else {
        format!("{base_path}/{trimmed_path}")
    };
    base.set_path(&joined_path);
    Ok(base)
}

fn truncate_message(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 180 {
        return trimmed.into();
    }
    let truncated: String = trimmed.chars().take(177).collect();
    format!("{truncated}...")
}

fn workspace_path_entries(workspace: &WorkspaceDetail) -> Vec<LineageEntry> {
    workspace
        .path
        .iter()
        .map(|item| LineageEntry {
            code: item.workspace_code.clone(),
            name: item.name.clone(),
        })
        .collect()
}

fn map_workspace_error(error: WorkspaceError) -> ModelError {
    match error {
        WorkspaceError::StorageFailure => ModelError::StorageFailure,
        WorkspaceError::NotFound => ModelError::InvalidWorkspace,
        _ => ModelError::InvalidWorkspace,
    }
}
