use serde_json::{Value, json};

use crate::{
    domain::resource::{
        PLATFORM_SCOPE_CODE, ResourceDefinition, ResourceDefinitionKey, ResourceDefinitionStatus,
        ResourceError, ResourceLifecycleStatus, ResourceScopeKind, ResourceValidationStatus,
    },
    shared::time::current_timestamp,
};

use super::{ResourceService, validation::validate_definition_content};

impl ResourceService {
    pub fn bootstrap_seed_definitions(&self) -> Result<(), ResourceError> {
        let existing_codes = self
            .repository(|repository| repository.list_definitions())?
            .into_iter()
            .filter(|definition| {
                definition.key.scope_kind == ResourceScopeKind::Platform
                    && definition.key.scope_code == PLATFORM_SCOPE_CODE
            })
            .map(|definition| definition.key.definition_code)
            .collect::<std::collections::BTreeSet<_>>();
        for seed in builtin_definitions() {
            if existing_codes.contains(seed.code) {
                continue;
            }
            let now = current_timestamp();
            let definition = ResourceDefinition {
                key: ResourceDefinitionKey {
                    scope_kind: ResourceScopeKind::Platform,
                    scope_code: PLATFORM_SCOPE_CODE.to_string(),
                    definition_code: seed.code.to_string(),
                    definition_version: "v1-000".to_string(),
                },
                name: seed.name.to_string(),
                description: seed.description.to_string(),
                definition_status: ResourceDefinitionStatus::Enabled,
                allow_inheritance: true,
                attributes_schema: seed.attributes_schema,
                binary_schema: seed.binary_schema,
                binary_storage_profile: "@default".to_string(),
                draft_revision: 0,
                validation_status: ResourceValidationStatus::Valid,
                lifecycle_status: ResourceLifecycleStatus::Active,
                created_at: now.clone(),
                updated_at: now.clone(),
                published_at: Some(now),
            };
            if !validate_definition_content(&definition, &self.config.limits, &self.storage)?
                .is_empty()
            {
                return Err(ResourceError::InvalidSchema);
            }
            self.repository(|repository| repository.insert_definition(&definition))?;
        }
        Ok(())
    }
}

struct BuiltinDefinition {
    code: &'static str,
    name: &'static str,
    description: &'static str,
    attributes_schema: Value,
    binary_schema: Value,
}

fn builtin_definitions() -> Vec<BuiltinDefinition> {
    vec![
        BuiltinDefinition {
            code: "FILE",
            name: "File",
            description: "A managed file resource.",
            attributes_schema: file_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 1 }),
        },
        BuiltinDefinition {
            code: "IMAGE",
            name: "Image",
            description: "A managed image resource.",
            attributes_schema: file_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 10 }),
        },
        BuiltinDefinition {
            code: "PDF",
            name: "PDF",
            description: "A managed PDF resource.",
            attributes_schema: file_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 1 }),
        },
        connection_definition("MYSQL_CONNECTION", "MySQL connection"),
        connection_definition("POSTGRESQL_CONNECTION", "PostgreSQL connection"),
        connection_definition("REDIS_CONNECTION", "Redis connection"),
        connection_definition("KAFKA_CONNECTION", "Kafka connection"),
    ]
}

fn file_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {
            "fileName": { "type": "string", "maxLength": 255 },
            "sizeBytes": { "type": "integer", "minimum": 0 }
        },
        "required": ["fileName", "sizeBytes"],
        "additionalProperties": false
    })
}

fn connection_definition(code: &'static str, name: &'static str) -> BuiltinDefinition {
    BuiltinDefinition {
        code,
        name,
        description: "A managed connection configuration.",
        attributes_schema: json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "host": { "type": "string", "minLength": 1 },
                "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
                "username": { "type": "string", "format": "tron-variable-ref" },
                "password": { "type": "string", "format": "tron-variable-ref" },
                "database": { "type": "string" }
            },
            "required": ["host", "port"],
            "additionalProperties": false
        }),
        binary_schema: json!({ "minCount": 0, "maxCount": 0 }),
    }
}
