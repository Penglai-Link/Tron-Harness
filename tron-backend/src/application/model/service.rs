use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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
mod enabled_state;
mod model_factory;
mod platform_use_cases;
mod repository_access;
mod verification;
mod verification_flow;
mod workspace_catalog;
mod workspace_mapping;
mod workspace_use_cases;

use self::{catalog_resolution::*, model_factory::*, verification::*, workspace_mapping::*};

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
    pub responses_stateful_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub max_retries: Option<u32>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeModelConfig {
    pub model_code: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key: String,
    pub model_name: String,
    pub thinking_enabled: bool,
    pub responses_stateful_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub max_retries: Option<u32>,
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
    responses_stateful_enabled: bool,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    context_window_tokens: Option<u32>,
    top_p: Option<f64>,
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
}
