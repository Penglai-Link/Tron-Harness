use crate::domain::resource::{
    Resource, ResourceActivationResult, ResourceBinaryMapping, ResourceDefinition,
    ResourceDefinitionKey, ResourceError, ResourceKey,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivationOutcome {
    pub activation_result: ResourceActivationResult,
    pub previous_active_version: Option<String>,
}

pub trait ResourceRepository: Send {
    fn list_definitions(&mut self) -> Result<Vec<ResourceDefinition>, ResourceError>;
    fn list_resources(&mut self) -> Result<Vec<Resource>, ResourceError>;
    fn list_binary_mappings(&mut self) -> Result<Vec<ResourceBinaryMapping>, ResourceError>;
    fn insert_definition(&mut self, definition: &ResourceDefinition) -> Result<(), ResourceError>;
    fn upgrade_builtin_definition(
        &mut self,
        expected_legacy: &ResourceDefinition,
        replacement: &ResourceDefinition,
    ) -> Result<bool, ResourceError>;
    fn update_definition_draft(
        &mut self,
        definition: &ResourceDefinition,
        expected_draft_revision: i64,
    ) -> Result<(), ResourceError>;
    fn activate_definition(
        &mut self,
        key: &ResourceDefinitionKey,
        expected_draft_revision: Option<i64>,
        expected_active_version: Option<&str>,
        published_at: &str,
    ) -> Result<ResourceActivationOutcome, ResourceError>;
    fn discard_definition_draft(
        &mut self,
        key: &ResourceDefinitionKey,
        expected_draft_revision: i64,
    ) -> Result<bool, ResourceError>;
    fn commit_active_definition(
        &mut self,
        definition: &ResourceDefinition,
        expected_local_active_version: Option<&str>,
        expected_effective_source: &ResourceDefinitionKey,
    ) -> Result<ResourceActivationOutcome, ResourceError>;
    fn delete_definition_group(
        &mut self,
        scope_kind: crate::domain::resource::ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
        expected_active_version: &str,
    ) -> Result<bool, ResourceError>;
    fn definition_resource_reference_count(
        &mut self,
        scope_kind: crate::domain::resource::ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
    ) -> Result<usize, ResourceError>;
    fn insert_resource(
        &mut self,
        resource: &Resource,
        mappings: &[ResourceBinaryMapping],
    ) -> Result<(), ResourceError>;
    fn update_resource_draft(
        &mut self,
        resource: &Resource,
        mappings: &[ResourceBinaryMapping],
        expected_draft_revision: i64,
    ) -> Result<(), ResourceError>;
    fn activate_resource(
        &mut self,
        key: &ResourceKey,
        expected_draft_revision: Option<i64>,
        expected_active_version: Option<&str>,
        published_at: &str,
    ) -> Result<ResourceActivationOutcome, ResourceError>;
    fn discard_resource_draft(
        &mut self,
        key: &ResourceKey,
        expected_draft_revision: i64,
    ) -> Result<(bool, Vec<String>), ResourceError>;
    fn commit_active_resource(
        &mut self,
        resource: &Resource,
        mappings: &[ResourceBinaryMapping],
        expected_local_active_version: Option<&str>,
        expected_effective_source: Option<&ResourceKey>,
        expected_enabled_definition: Option<&ResourceDefinitionKey>,
    ) -> Result<ResourceActivationOutcome, ResourceError>;
    fn delete_resource_group(
        &mut self,
        scope_kind: crate::domain::resource::ResourceScopeKind,
        scope_code: &str,
        resource_code: &str,
        expected_active_version: Option<&str>,
        expected_draft_version: Option<&str>,
        expected_draft_revision: Option<i64>,
    ) -> Result<(bool, Vec<String>), ResourceError>;
    fn locator_reference_count(&mut self, locator: &str) -> Result<usize, ResourceError>;
}

pub trait ObjectStorage: Send + Sync {
    fn put_file(&self, object_key: &str, source: &std::path::Path) -> Result<u64, ResourceError>;
    fn put_reader(
        &self,
        object_key: &str,
        source: &mut dyn std::io::Read,
    ) -> Result<u64, ResourceError>;
    fn get_file(&self, object_key: &str, target: &std::path::Path) -> Result<u64, ResourceError>;
    fn get_writer(
        &self,
        object_key: &str,
        target: &mut dyn std::io::Write,
    ) -> Result<u64, ResourceError>;
    fn delete(&self, object_key: &str) -> Result<(), ResourceError>;
    fn exists(&self, object_key: &str) -> Result<bool, ResourceError>;
}
