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

/// Page queries and commands below are intentionally separate from the Agent Tool
/// DTOs above.  The Tool contract groups operations for an agent, while the HTTP
/// management API exposes one page-oriented query or lifecycle action at a time.
#[derive(Debug, Clone)]
pub struct ResourceDefinitionVersionPageQuery {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub search: Option<String>,
    pub definition_code: Option<String>,
    pub definition_status: Option<ResourceDefinitionStatus>,
    pub lifecycle_status: Option<ResourceLifecycleStatus>,
    pub validation_status: Option<ResourceValidationStatus>,
    pub source_kind: Option<ResourceSourceKind>,
    pub current: usize,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionVersionGetQuery {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionCreateDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub source_definition_version: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub definition_status: Option<ResourceDefinitionStatus>,
    pub allow_inheritance: Option<bool>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionUpdateDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_draft_revision: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub definition_status: Option<ResourceDefinitionStatus>,
    pub allow_inheritance: Option<bool>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionDiscardDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_draft_revision: i64,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionPublishDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_draft_revision: i64,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionActivateVersionCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_active_definition_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceVersionPageQuery {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub search: Option<String>,
    pub resource_code: Option<String>,
    pub definition_code: Option<String>,
    pub definition_status: Option<ResourceDefinitionStatus>,
    pub lifecycle_status: Option<ResourceLifecycleStatus>,
    pub validation_status: Option<ResourceValidationStatus>,
    pub source_kind: Option<ResourceSourceKind>,
    pub current: usize,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceVersionGetQuery {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
}

#[derive(Debug, Clone)]
pub struct ResourceCreateDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub source_resource_version: Option<String>,
    pub definition_code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ResourceUpdateDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ResourceDiscardDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: i64,
}

#[derive(Debug, Clone)]
pub struct ResourcePublishDraftCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: i64,
}

#[derive(Debug, Clone)]
pub struct ResourceActivateVersionCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_active_resource_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceHttpBinaryUploadCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: i64,
    pub binary_name: String,
}

#[derive(Debug, Clone)]
pub struct ResourceHttpBinaryRemoveCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: i64,
    pub binary_name: String,
}

#[derive(Debug, Clone)]
pub struct ResourceHttpBinaryDownloadCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub binary_name: String,
}

/// A browser-facing commit always produces an immutable ACTIVE snapshot.  It
/// intentionally differs from the explicit Draft commands above, which remain
/// available only for incomplete work.
#[derive(Debug, Clone)]
pub struct ResourceCommitCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub definition_code: Option<String>,
    pub expected_active_resource_version: Option<String>,
    pub name: String,
    pub description: String,
    pub attributes: Value,
    pub allow_inheritance: bool,
    pub removed_binary_names: Vec<String>,
}

#[derive(Debug)]
pub struct ResourceCommitBinary {
    pub binary_name: String,
    pub source: std::fs::File,
}

#[derive(Debug, Clone)]
pub struct ResourceDeleteCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub expected_active_resource_version: Option<String>,
    pub expected_draft_resource_version: Option<String>,
    pub expected_draft_revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDeleteResult {
    pub resource_code: String,
    pub deleted: bool,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionSetEnabledCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub enabled: bool,
    pub expected_active_definition_version: String,
}

#[derive(Debug, Clone)]
pub struct ResourceDefinitionDeleteCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub expected_active_definition_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionDeleteResult {
    pub definition_code: String,
    pub deleted: bool,
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
    pub local_draft: Option<ResourceDefinitionDraftRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionDraftRecord {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub name: String,
    pub description: String,
    pub definition_status: ResourceDefinitionStatus,
    pub allow_inheritance: bool,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub validation_status: ResourceValidationStatus,
    pub draft_revision: i64,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDraftSummary {
    pub resource_version: String,
    pub draft_revision: i64,
    pub validation_status: ResourceValidationStatus,
    pub name: Option<String>,
    pub description: Option<String>,
    pub definition_scope_kind: Option<ResourceScopeKind>,
    pub definition_scope_code: Option<String>,
    pub definition_code: Option<String>,
    pub definition_version: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
    pub lifecycle_status: Option<ResourceLifecycleStatus>,
    pub binaries: Option<Vec<ResourceBinarySummary>>,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResourceDefinitionEffectiveBinaryPolicy {
    pub min_count: usize,
    pub max_count: usize,
    pub max_size_bytes: u64,
    pub max_name_length: usize,
    pub name_pattern: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResourceDefinitionVersionView {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub source_kind: ResourceSourceKind,
    pub built_in: bool,
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
    pub effective_binary_policy: Option<ResourceDefinitionEffectiveBinaryPolicy>,
    pub binary_storage_profile: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResourceDefinitionVersionPage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceDefinitionVersionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResourceVersionView {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub source_kind: ResourceSourceKind,
    pub resource_code: String,
    pub resource_version: String,
    pub definition_scope_kind: ResourceScopeKind,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub definition_name: String,
    pub definition_status: ResourceDefinitionStatus,
    pub name: String,
    pub description: String,
    pub attributes: Option<Value>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
    pub allow_inheritance: bool,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub validation_status: ResourceValidationStatus,
    pub draft_revision: Option<i64>,
    pub binaries: Option<Vec<ResourceBinarySummary>>,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResourceVersionPage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceVersionView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHttpBinaryDownload {
    pub resource_version: String,
    pub binary_name: String,
    pub byte_size: u64,
}
