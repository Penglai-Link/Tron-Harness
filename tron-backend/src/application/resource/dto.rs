use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::resource::{
    ResourceActivationResult, ResourceDefinitionStatus, ResourceLifecycleStatus, ResourceScopeKind,
    ResourceSourceKind, ResourceValidationStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceDefinitionDraftAction {
    Create,
    Update,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceDraftAction {
    Create,
    Update,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceBinaryAction {
    Upload,
    Download,
    Remove,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionQuery {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub search: Option<String>,
    pub definition_code: Option<String>,
    pub definition_version: Option<String>,
    pub definition_status: Option<ResourceDefinitionStatus>,
    pub lifecycle_status: Option<ResourceLifecycleStatus>,
    pub include_history: bool,
    pub current: usize,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub action: ResourceDefinitionDraftAction,
    pub definition_code: String,
    pub definition_version: Option<String>,
    pub expected_draft_revision: Option<i64>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub definition_status: Option<ResourceDefinitionStatus>,
    pub allow_inheritance: Option<bool>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionPublishCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_draft_revision: Option<i64>,
    pub expected_active_definition_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceQuery {
    pub workspace_code: Option<String>,
    pub search: Option<String>,
    pub definition_code: Option<String>,
    pub resource_code: Option<String>,
    pub resource_version: Option<String>,
    pub include_history: bool,
    pub current: usize,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceDraftCommand {
    pub action: ResourceDraftAction,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: Option<String>,
    pub expected_draft_revision: Option<i64>,
    pub definition_code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ResourcePublishCommand {
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: Option<i64>,
    pub expected_active_resource_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceBinaryCommand {
    pub action: ResourceBinaryAction,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: Option<String>,
    pub expected_draft_revision: Option<i64>,
    pub binary_name: String,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub overwrite: bool,
    pub agent_local_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceValidationIssue {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionRecord {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub source_kind: ResourceSourceKind,
    pub definition_code: String,
    pub definition_version: String,
    pub name: String,
    pub description: String,
    pub definition_status: ResourceDefinitionStatus,
    pub allow_inheritance: bool,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub validation_status: ResourceValidationStatus,
    pub draft_revision: Option<i64>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionPage {
    pub result_kind: String,
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceDefinitionRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionDraftResult {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub draft_revision: i64,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub validation_status: ResourceValidationStatus,
    pub validation_errors: Vec<ResourceValidationIssue>,
    pub discarded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionActivationResult {
    pub definition_code: String,
    pub definition_version: String,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub activation_result: ResourceActivationResult,
    pub previous_active_definition_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinarySummary {
    pub binary_name: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRecord {
    pub resource_code: String,
    pub resource_version: Option<String>,
    pub source_kind: ResourceSourceKind,
    pub definition_scope_kind: ResourceScopeKind,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub name: String,
    pub description: String,
    pub attributes: Option<Value>,
    pub allow_inheritance: bool,
    pub lifecycle_status: Option<ResourceLifecycleStatus>,
    pub validation_status: Option<ResourceValidationStatus>,
    pub draft_revision: Option<i64>,
    pub binaries: Option<Vec<ResourceBinarySummary>>,
    pub local_draft: Option<ResourceDraftSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDraftSummary {
    pub resource_version: String,
    pub draft_revision: i64,
    pub validation_status: ResourceValidationStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDraftResult {
    pub draft_result: String,
    pub resource_code: String,
    pub resource_version: String,
    pub definition_scope_kind: ResourceScopeKind,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub draft_revision: i64,
    pub validation_status: ResourceValidationStatus,
    pub validation_errors: Vec<ResourceValidationIssue>,
    pub discarded: Option<bool>,
    pub resource_removed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceActivationOutput {
    pub resource_code: String,
    pub resource_version: String,
    pub definition_scope_kind: ResourceScopeKind,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub activation_result: ResourceActivationResult,
    pub previous_active_resource_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinaryResult {
    pub action: String,
    pub resource_code: String,
    pub resource_version: Option<String>,
    pub binary_name: String,
    pub byte_size: Option<u64>,
    pub target_path: Option<String>,
    pub draft_revision: Option<i64>,
    pub validation_status: Option<ResourceValidationStatus>,
    pub validation_errors: Vec<ResourceValidationIssue>,
}
