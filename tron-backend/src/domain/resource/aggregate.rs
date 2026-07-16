use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    ResourceDefinitionStatus, ResourceLifecycleStatus, ResourceScopeKind, ResourceValidationStatus,
};

pub const PLATFORM_SCOPE_CODE: &str = "PLATFORM";
pub const DEFAULT_BINARY_STORAGE_PROFILE: &str = "@default";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionKey {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub definition_code: String,
    pub definition_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinitionGroupKey {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub definition_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinition {
    pub key: ResourceDefinitionKey,
    pub name: String,
    pub description: String,
    pub definition_status: ResourceDefinitionStatus,
    pub allow_inheritance: bool,
    pub attributes_schema: Value,
    pub binary_schema: Value,
    pub binary_storage_profile: String,
    pub draft_revision: i64,
    pub validation_status: ResourceValidationStatus,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceKey {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub resource_code: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGroupKey {
    pub scope_kind: ResourceScopeKind,
    pub scope_code: String,
    pub resource_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub key: ResourceKey,
    pub definition_key: ResourceDefinitionKey,
    pub based_on_resource_version: Option<String>,
    pub name: String,
    pub description: String,
    pub attributes: Value,
    pub allow_inheritance: bool,
    pub draft_revision: i64,
    pub validation_status: ResourceValidationStatus,
    pub lifecycle_status: ResourceLifecycleStatus,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceBinaryMapping {
    pub resource_key: ResourceKey,
    pub binary_name: String,
    pub storage_locator: String,
    pub byte_size: u64,
    pub created_at: String,
}

impl ResourceDefinition {
    pub fn group_key(&self) -> ResourceDefinitionGroupKey {
        ResourceDefinitionGroupKey {
            scope_kind: self.key.scope_kind,
            scope_code: self.key.scope_code.clone(),
            definition_code: self.key.definition_code.clone(),
        }
    }
}

impl Resource {
    pub fn group_key(&self) -> ResourceGroupKey {
        ResourceGroupKey {
            scope_kind: self.key.scope_kind,
            scope_code: self.key.scope_code.clone(),
            resource_code: self.key.resource_code.clone(),
        }
    }
}
