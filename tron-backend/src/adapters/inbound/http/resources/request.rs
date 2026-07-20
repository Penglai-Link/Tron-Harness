use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        ResourceActivateVersionCommand, ResourceCommitCommand, ResourceCreateDraftCommand,
        ResourceDefinitionActivateVersionCommand, ResourceDefinitionCreateDraftCommand,
        ResourceDefinitionDeleteCommand, ResourceDefinitionDiscardDraftCommand,
        ResourceDefinitionPublishDraftCommand, ResourceDefinitionSetEnabledCommand,
        ResourceDefinitionUpdateDraftCommand, ResourceDefinitionVersionGetQuery,
        ResourceDefinitionVersionPageQuery, ResourceDeleteCommand, ResourceDiscardDraftCommand,
        ResourceHttpBinaryDownloadCommand, ResourceHttpBinaryRemoveCommand,
        ResourceHttpBinaryUploadCommand, ResourcePublishDraftCommand, ResourceUpdateDraftCommand,
        ResourceVersionGetQuery, ResourceVersionPageQuery,
    },
    domain::resource::{
        ResourceDefinitionStatus, ResourceError, ResourceLifecycleStatus, ResourceScopeKind,
        ResourceSourceKind, ResourceValidationStatus,
    },
};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScopeCode {
    Platform,
    Workspace,
}

impl From<ScopeCode> for ResourceScopeKind {
    fn from(value: ScopeCode) -> Self {
        match value {
            ScopeCode::Platform => Self::Platform,
            ScopeCode::Workspace => Self::Workspace,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleCode {
    Draft,
    Active,
    Historical,
}

impl From<LifecycleCode> for ResourceLifecycleStatus {
    fn from(value: LifecycleCode) -> Self {
        match value {
            LifecycleCode::Draft => Self::Draft,
            LifecycleCode::Active => Self::Active,
            LifecycleCode::Historical => Self::Historical,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationCode {
    Valid,
    Invalid,
}

impl From<ValidationCode> for ResourceValidationStatus {
    fn from(value: ValidationCode) -> Self {
        match value {
            ValidationCode::Valid => Self::Valid,
            ValidationCode::Invalid => Self::Invalid,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DefinitionStatusCode {
    Enabled,
    Disabled,
}

impl From<DefinitionStatusCode> for ResourceDefinitionStatus {
    fn from(value: DefinitionStatusCode) -> Self {
        match value {
            DefinitionStatusCode::Enabled => Self::Enabled,
            DefinitionStatusCode::Disabled => Self::Disabled,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceCode {
    Platform,
    Local,
    Inherited,
    Override,
}

impl From<SourceCode> for ResourceSourceKind {
    fn from(value: SourceCode) -> Self {
        match value {
            SourceCode::Platform => Self::Platform,
            SourceCode::Local => Self::Local,
            SourceCode::Inherited => Self::Inherited,
            SourceCode::Override => Self::Override,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionPageRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub search: Option<String>,
    pub definition_code: Option<String>,
    pub definition_status: Option<DefinitionStatusCode>,
    pub lifecycle_status: Option<LifecycleCode>,
    pub validation_status: Option<ValidationCode>,
    pub source_kind: Option<SourceCode>,
    pub current: usize,
    pub size: usize,
}

impl From<DefinitionPageRequest> for ResourceDefinitionVersionPageQuery {
    fn from(value: DefinitionPageRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            search: value.search,
            definition_code: value.definition_code,
            definition_status: value.definition_status.map(Into::into),
            lifecycle_status: value.lifecycle_status.map(Into::into),
            validation_status: value.validation_status.map(Into::into),
            source_kind: value.source_kind.map(Into::into),
            current: value.current,
            size: value.size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionGetRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
}

impl From<DefinitionGetRequest> for ResourceDefinitionVersionGetQuery {
    fn from(value: DefinitionGetRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionCreateDraftRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub source_definition_version: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub definition_status: Option<DefinitionStatusCode>,
    pub allow_inheritance: Option<bool>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
}

impl From<DefinitionCreateDraftRequest> for ResourceDefinitionCreateDraftCommand {
    fn from(value: DefinitionCreateDraftRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            source_definition_version: value.source_definition_version,
            name: value.name,
            description: value.description,
            definition_status: value.definition_status.map(Into::into),
            allow_inheritance: value.allow_inheritance,
            attributes_schema: value.attributes_schema,
            binary_schema: value.binary_schema,
            binary_storage_profile: value.binary_storage_profile,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionUpdateDraftRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_draft_revision: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub definition_status: Option<DefinitionStatusCode>,
    pub allow_inheritance: Option<bool>,
    pub attributes_schema: Option<Value>,
    pub binary_schema: Option<Value>,
    pub binary_storage_profile: Option<String>,
}

impl TryFrom<DefinitionUpdateDraftRequest> for ResourceDefinitionUpdateDraftCommand {
    type Error = ResourceError;

    fn try_from(value: DefinitionUpdateDraftRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
            name: value.name,
            description: value.description,
            definition_status: value.definition_status.map(Into::into),
            allow_inheritance: value.allow_inheritance,
            attributes_schema: value.attributes_schema,
            binary_schema: value.binary_schema,
            binary_storage_profile: value.binary_storage_profile,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionDraftTargetRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_draft_revision: String,
}

impl TryFrom<DefinitionDraftTargetRequest> for ResourceDefinitionDiscardDraftCommand {
    type Error = ResourceError;

    fn try_from(value: DefinitionDraftTargetRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
        })
    }
}

impl TryFrom<DefinitionDraftTargetRequest> for ResourceDefinitionPublishDraftCommand {
    type Error = ResourceError;

    fn try_from(value: DefinitionDraftTargetRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionActivateVersionRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub definition_version: String,
    pub expected_active_definition_version: Option<String>,
}

impl From<DefinitionActivateVersionRequest> for ResourceDefinitionActivateVersionCommand {
    fn from(value: DefinitionActivateVersionRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            definition_version: value.definition_version,
            expected_active_definition_version: value.expected_active_definition_version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionSetEnabledRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub enabled: bool,
    pub expected_active_definition_version: String,
}

impl From<DefinitionSetEnabledRequest> for ResourceDefinitionSetEnabledCommand {
    fn from(value: DefinitionSetEnabledRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            enabled: value.enabled,
            expected_active_definition_version: value.expected_active_definition_version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionDeleteRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub definition_code: String,
    pub expected_active_definition_version: String,
}

impl From<DefinitionDeleteRequest> for ResourceDefinitionDeleteCommand {
    fn from(value: DefinitionDeleteRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            definition_code: value.definition_code,
            expected_active_definition_version: value.expected_active_definition_version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePageRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub search: Option<String>,
    pub resource_code: Option<String>,
    pub definition_code: Option<String>,
    pub definition_status: Option<DefinitionStatusCode>,
    pub lifecycle_status: Option<LifecycleCode>,
    pub validation_status: Option<ValidationCode>,
    pub source_kind: Option<SourceCode>,
    pub current: usize,
    pub size: usize,
}

impl From<ResourcePageRequest> for ResourceVersionPageQuery {
    fn from(value: ResourcePageRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            search: value.search,
            resource_code: value.resource_code,
            definition_code: value.definition_code,
            definition_status: value.definition_status.map(Into::into),
            lifecycle_status: value.lifecycle_status.map(Into::into),
            validation_status: value.validation_status.map(Into::into),
            source_kind: value.source_kind.map(Into::into),
            current: value.current,
            size: value.size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceGetRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
}

impl From<ResourceGetRequest> for ResourceVersionGetQuery {
    fn from(value: ResourceGetRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceCommitRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub definition_code: Option<String>,
    pub expected_active_resource_version: Option<String>,
    pub name: String,
    pub description: String,
    pub attributes: Value,
    pub allow_inheritance: bool,
    #[serde(default)]
    pub removed_binary_names: Vec<String>,
}

impl From<ResourceCommitRequest> for ResourceCommitCommand {
    fn from(value: ResourceCommitRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            definition_code: value.definition_code,
            expected_active_resource_version: value.expected_active_resource_version,
            name: value.name,
            description: value.description,
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance,
            removed_binary_names: value.removed_binary_names,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDeleteRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub expected_active_resource_version: Option<String>,
    pub expected_draft_resource_version: Option<String>,
    pub expected_draft_revision: Option<String>,
}

impl TryFrom<ResourceDeleteRequest> for ResourceDeleteCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceDeleteRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            expected_active_resource_version: value.expected_active_resource_version,
            expected_draft_resource_version: value.expected_draft_resource_version,
            expected_draft_revision: value
                .expected_draft_revision
                .as_deref()
                .map(parse_draft_revision)
                .transpose()?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceCreateDraftRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub source_resource_version: Option<String>,
    pub definition_code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
}

impl From<ResourceCreateDraftRequest> for ResourceCreateDraftCommand {
    fn from(value: ResourceCreateDraftRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            source_resource_version: value.source_resource_version,
            definition_code: value.definition_code,
            name: value.name,
            description: value.description,
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceUpdateDraftRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
}

impl TryFrom<ResourceUpdateDraftRequest> for ResourceUpdateDraftCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceUpdateDraftRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
            name: value.name,
            description: value.description,
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDraftTargetRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: String,
}

impl TryFrom<ResourceDraftTargetRequest> for ResourceDiscardDraftCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceDraftTargetRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
        })
    }
}

impl TryFrom<ResourceDraftTargetRequest> for ResourcePublishDraftCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceDraftTargetRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceActivateVersionRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_active_resource_version: Option<String>,
}

impl From<ResourceActivateVersionRequest> for ResourceActivateVersionCommand {
    fn from(value: ResourceActivateVersionRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            expected_active_resource_version: value.expected_active_resource_version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceBinaryRemoveRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub expected_draft_revision: String,
    pub binary_name: String,
}

impl TryFrom<ResourceBinaryRemoveRequest> for ResourceHttpBinaryRemoveCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceBinaryRemoveRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            expected_draft_revision: parse_draft_revision(&value.expected_draft_revision)?,
            binary_name: value.binary_name,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceBinaryDownloadRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub resource_version: String,
    pub binary_name: String,
}

impl From<ResourceBinaryDownloadRequest> for ResourceHttpBinaryDownloadCommand {
    fn from(value: ResourceBinaryDownloadRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            resource_version: value.resource_version,
            binary_name: value.binary_name,
        }
    }
}

pub fn binary_upload_command(
    scope_kind: &str,
    workspace_code: Option<String>,
    resource_code: String,
    resource_version: String,
    expected_draft_revision: &str,
    binary_name: String,
) -> Result<ResourceHttpBinaryUploadCommand, ResourceError> {
    Ok(ResourceHttpBinaryUploadCommand {
        scope_kind: parse_scope_kind(scope_kind)?,
        workspace_code,
        resource_code,
        resource_version,
        expected_draft_revision: parse_draft_revision(expected_draft_revision)?,
        binary_name,
    })
}

fn parse_scope_kind(value: &str) -> Result<ResourceScopeKind, ResourceError> {
    match value.trim() {
        "PLATFORM" => Ok(ResourceScopeKind::Platform),
        "WORKSPACE" => Ok(ResourceScopeKind::Workspace),
        _ => Err(ResourceError::InvalidScope),
    }
}

fn parse_draft_revision(value: &str) -> Result<i64, ResourceError> {
    value
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|revision| *revision > 0)
        .ok_or(ResourceError::InvalidInput)
}
