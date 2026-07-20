use serde::Serialize;
use serde_json::Value;

use crate::{
    application::resource::{
        ResourceActivationOutput, ResourceBinaryResult, ResourceDefinitionActivationResult,
        ResourceDefinitionDeleteResult, ResourceDefinitionDraftResult,
        ResourceDefinitionVersionPage, ResourceDefinitionVersionView, ResourceDeleteResult,
        ResourceDraftResult, ResourceValidationIssue, ResourceVersionPage, ResourceVersionView,
    },
    domain::resource::{
        ResourceActivationResult, ResourceDefinitionStatus, ResourceLifecycleStatus,
        ResourceScopeKind, ResourceSourceKind, ResourceValidationStatus,
    },
    shared::time::{option_timestamp_to_millis, timestamp_to_millis},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceHttpApiResponse<T> {
    pub code: String,
    pub message: String,
    pub data: T,
}

impl<T> ResourceHttpApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: "0".to_string(),
            message: "ok".to_string(),
            data,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ResourceHttpEmptyData {}

impl ResourceHttpApiResponse<ResourceHttpEmptyData> {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: ResourceHttpEmptyData {},
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionDeleteResponse {
    pub definition_code: String,
    pub deleted: bool,
}

impl From<ResourceDefinitionDeleteResult> for DefinitionDeleteResponse {
    fn from(value: ResourceDefinitionDeleteResult) -> Self {
        Self {
            definition_code: value.definition_code,
            deleted: value.deleted,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDeleteResponse {
    pub resource_code: String,
    pub deleted: bool,
}

impl From<ResourceDeleteResult> for ResourceDeleteResponse {
    fn from(value: ResourceDeleteResult) -> Self {
        Self {
            resource_code: value.resource_code,
            deleted: value.deleted,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionVersionPageResponse {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<DefinitionVersionResponse>,
}

impl From<ResourceDefinitionVersionPage> for DefinitionVersionPageResponse {
    fn from(value: ResourceDefinitionVersionPage) -> Self {
        Self {
            current: value.current,
            size: value.size,
            total: value.total,
            records: value.records.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveBinaryPolicyResponse {
    pub min_count: usize,
    pub max_count: usize,
    pub max_size_bytes: u64,
    pub max_name_length: usize,
    pub name_pattern: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionVersionResponse {
    pub scope_kind: &'static str,
    pub scope_code: String,
    pub source_kind: &'static str,
    pub built_in: bool,
    pub definition_code: String,
    pub definition_version: String,
    pub name: String,
    pub description: String,
    pub definition_status: &'static str,
    pub allow_inheritance: bool,
    pub lifecycle_status: &'static str,
    pub validation_status: &'static str,
    pub draft_revision: Option<String>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub effective_binary_policy: Option<EffectiveBinaryPolicyResponse>,
    pub binary_storage_profile: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub published_at: Option<i64>,
}

impl From<ResourceDefinitionVersionView> for DefinitionVersionResponse {
    fn from(value: ResourceDefinitionVersionView) -> Self {
        Self {
            scope_kind: scope_code(value.scope_kind),
            scope_code: value.scope_code,
            source_kind: source_code(value.source_kind),
            built_in: value.built_in,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            name: value.name,
            description: value.description,
            definition_status: definition_status_code(value.definition_status),
            allow_inheritance: value.allow_inheritance,
            lifecycle_status: lifecycle_code(value.lifecycle_status),
            validation_status: validation_code(value.validation_status),
            draft_revision: value.draft_revision.map(|revision| revision.to_string()),
            attributes_schema: value.attributes_schema,
            binary_schema: value.binary_schema,
            effective_binary_policy: value.effective_binary_policy.map(|policy| {
                EffectiveBinaryPolicyResponse {
                    min_count: policy.min_count,
                    max_count: policy.max_count,
                    max_size_bytes: policy.max_size_bytes,
                    max_name_length: policy.max_name_length,
                    name_pattern: policy.name_pattern,
                }
            }),
            binary_storage_profile: value.binary_storage_profile,
            created_at: timestamp_to_millis(&value.created_at),
            updated_at: timestamp_to_millis(&value.updated_at),
            published_at: option_timestamp_to_millis(value.published_at.as_deref()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceVersionPageResponse {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceVersionResponse>,
}

impl From<ResourceVersionPage> for ResourceVersionPageResponse {
    fn from(value: ResourceVersionPage) -> Self {
        Self {
            current: value.current,
            size: value.size,
            total: value.total,
            records: value.records.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceVersionResponse {
    pub scope_kind: &'static str,
    pub scope_code: String,
    pub source_kind: &'static str,
    pub resource_code: String,
    pub resource_version: String,
    pub definition_scope_kind: &'static str,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub definition_name: String,
    pub definition_status: &'static str,
    pub name: String,
    pub description: String,
    pub attributes: Option<Value>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
    pub allow_inheritance: bool,
    pub lifecycle_status: &'static str,
    pub validation_status: &'static str,
    pub draft_revision: Option<String>,
    pub binaries: Option<Vec<ResourceBinaryResponse>>,
    pub created_at: i64,
    pub updated_at: i64,
    pub published_at: Option<i64>,
}

impl From<ResourceVersionView> for ResourceVersionResponse {
    fn from(value: ResourceVersionView) -> Self {
        Self {
            scope_kind: scope_code(value.scope_kind),
            scope_code: value.scope_code,
            source_kind: source_code(value.source_kind),
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            definition_scope_kind: scope_code(value.definition_scope_kind),
            definition_scope_code: value.definition_scope_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            definition_name: value.definition_name,
            definition_status: definition_status_code(value.definition_status),
            name: value.name,
            description: value.description,
            attributes: value.attributes,
            attributes_schema: value.attributes_schema,
            binary_schema: value.binary_schema,
            binary_storage_profile: value.binary_storage_profile,
            allow_inheritance: value.allow_inheritance,
            lifecycle_status: lifecycle_code(value.lifecycle_status),
            validation_status: validation_code(value.validation_status),
            draft_revision: value.draft_revision.map(|revision| revision.to_string()),
            binaries: value.binaries.map(|binaries| {
                binaries
                    .into_iter()
                    .map(|binary| ResourceBinaryResponse {
                        binary_name: binary.binary_name,
                        byte_size: binary.byte_size,
                    })
                    .collect()
            }),
            created_at: timestamp_to_millis(&value.created_at),
            updated_at: timestamp_to_millis(&value.updated_at),
            published_at: option_timestamp_to_millis(value.published_at.as_deref()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinaryResponse {
    pub binary_name: String,
    pub byte_size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssueResponse {
    pub path: String,
    pub message: String,
}

impl From<ResourceValidationIssue> for ValidationIssueResponse {
    fn from(value: ResourceValidationIssue) -> Self {
        Self {
            path: value.path,
            message: value.message,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionDraftActionResponse {
    pub scope_kind: &'static str,
    pub scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub draft_revision: String,
    pub lifecycle_status: &'static str,
    pub validation_status: &'static str,
    pub validation_errors: Vec<ValidationIssueResponse>,
    pub discarded: Option<bool>,
}

impl From<ResourceDefinitionDraftResult> for DefinitionDraftActionResponse {
    fn from(value: ResourceDefinitionDraftResult) -> Self {
        Self {
            scope_kind: scope_code(value.scope_kind),
            scope_code: value.scope_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            draft_revision: value.draft_revision.to_string(),
            lifecycle_status: lifecycle_code(value.lifecycle_status),
            validation_status: validation_code(value.validation_status),
            validation_errors: value
                .validation_errors
                .into_iter()
                .map(Into::into)
                .collect(),
            discarded: value.discarded,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionActivationResponse {
    pub definition_code: String,
    pub definition_version: String,
    pub lifecycle_status: &'static str,
    pub activation_result: &'static str,
    pub previous_active_definition_version: Option<String>,
}

impl From<ResourceDefinitionActivationResult> for DefinitionActivationResponse {
    fn from(value: ResourceDefinitionActivationResult) -> Self {
        Self {
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            lifecycle_status: lifecycle_code(value.lifecycle_status),
            activation_result: activation_code(value.activation_result),
            previous_active_definition_version: value.previous_active_definition_version,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDraftActionResponse {
    pub draft_result: String,
    pub resource_code: String,
    pub resource_version: String,
    pub definition_scope_kind: &'static str,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub draft_revision: String,
    pub validation_status: &'static str,
    pub validation_errors: Vec<ValidationIssueResponse>,
    pub discarded: Option<bool>,
    pub resource_removed: Option<bool>,
}

impl From<ResourceDraftResult> for ResourceDraftActionResponse {
    fn from(value: ResourceDraftResult) -> Self {
        Self {
            draft_result: value.draft_result,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            definition_scope_kind: scope_code(value.definition_scope_kind),
            definition_scope_code: value.definition_scope_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            draft_revision: value.draft_revision.to_string(),
            validation_status: validation_code(value.validation_status),
            validation_errors: value
                .validation_errors
                .into_iter()
                .map(Into::into)
                .collect(),
            discarded: value.discarded,
            resource_removed: value.resource_removed,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceActivationResponse {
    pub resource_code: String,
    pub resource_version: String,
    pub definition_scope_kind: &'static str,
    pub definition_scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
    pub lifecycle_status: &'static str,
    pub activation_result: &'static str,
    pub previous_active_resource_version: Option<String>,
}

impl From<ResourceActivationOutput> for ResourceActivationResponse {
    fn from(value: ResourceActivationOutput) -> Self {
        Self {
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            definition_scope_kind: scope_code(value.definition_scope_kind),
            definition_scope_code: value.definition_scope_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            lifecycle_status: lifecycle_code(value.lifecycle_status),
            activation_result: activation_code(value.activation_result),
            previous_active_resource_version: value.previous_active_resource_version,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinaryActionResponse {
    pub action: String,
    pub resource_code: String,
    pub resource_version: Option<String>,
    pub binary_name: String,
    pub byte_size: Option<u64>,
    pub draft_revision: Option<String>,
    pub validation_status: Option<&'static str>,
    pub validation_errors: Vec<ValidationIssueResponse>,
}

impl From<ResourceBinaryResult> for ResourceBinaryActionResponse {
    fn from(value: ResourceBinaryResult) -> Self {
        Self {
            action: value.action,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            binary_name: value.binary_name,
            byte_size: value.byte_size,
            draft_revision: value.draft_revision.map(|revision| revision.to_string()),
            validation_status: value.validation_status.map(validation_code),
            validation_errors: value
                .validation_errors
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

fn scope_code(value: ResourceScopeKind) -> &'static str {
    match value {
        ResourceScopeKind::Platform => "PLATFORM",
        ResourceScopeKind::Workspace => "WORKSPACE",
    }
}

fn source_code(value: ResourceSourceKind) -> &'static str {
    match value {
        ResourceSourceKind::Platform => "PLATFORM",
        ResourceSourceKind::Local => "LOCAL",
        ResourceSourceKind::Inherited => "INHERITED",
        ResourceSourceKind::Override => "OVERRIDE",
    }
}

fn lifecycle_code(value: ResourceLifecycleStatus) -> &'static str {
    match value {
        ResourceLifecycleStatus::Draft => "DRAFT",
        ResourceLifecycleStatus::Active => "ACTIVE",
        ResourceLifecycleStatus::Historical => "HISTORICAL",
    }
}

fn validation_code(value: ResourceValidationStatus) -> &'static str {
    match value {
        ResourceValidationStatus::Valid => "VALID",
        ResourceValidationStatus::Invalid => "INVALID",
    }
}

fn definition_status_code(value: ResourceDefinitionStatus) -> &'static str {
    match value {
        ResourceDefinitionStatus::Enabled => "ENABLED",
        ResourceDefinitionStatus::Disabled => "DISABLED",
    }
}

fn activation_code(value: ResourceActivationResult) -> &'static str {
    match value {
        ResourceActivationResult::DirectCommitted => "COMMITTED",
        ResourceActivationResult::DraftPublished => "DRAFT_PUBLISHED",
        ResourceActivationResult::HistoricalReactivated => "HISTORICAL_REACTIVATED",
        ResourceActivationResult::AlreadyActive => "ALREADY_ACTIVE",
    }
}
