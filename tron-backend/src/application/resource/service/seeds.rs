use serde_json::{Value, json};

use crate::{
    domain::resource::{
        DEFAULT_BINARY_STORAGE_PROFILE, PLATFORM_SCOPE_CODE, ResourceDefinition,
        ResourceDefinitionKey, ResourceDefinitionStatus, ResourceError, ResourceLifecycleStatus,
        ResourceScopeKind, ResourceValidationStatus,
    },
    shared::time::current_timestamp,
};

use super::{
    ResourceService,
    connection_schema::{endpoint_schema, host_schema, port_schema},
    connection_secret_schema::{previous_variable_password_seed, secret_reference_schema},
    definitions::next_definition_version,
    legacy_connection_definition::{legacy_connection_definition, legacy_file_schema},
    validation::validate_definition_content,
};

const DRAFT_7: &str = "http://json-schema.org/draft-07/schema#";

pub(super) fn is_builtin_definition_code(code: &str) -> bool {
    matches!(
        code,
        "FILE"
            | "IMAGE"
            | "PDF"
            | "MYSQL_CONNECTION"
            | "POSTGRESQL_CONNECTION"
            | "REDIS_CONNECTION"
            | "KAFKA_CONNECTION"
            | "SERVER_CONNECTION"
            | "SERVER_ACCOUNT"
    )
}

impl ResourceService {
    pub fn bootstrap_seed_definitions(&self) -> Result<(), ResourceError> {
        let seeds = builtin_definitions();
        let existing = self.repository(|repository| repository.list_definitions())?;
        let existing_codes = existing
            .iter()
            .filter(|definition| {
                definition.key.scope_kind == ResourceScopeKind::Platform
                    && definition.key.scope_code == PLATFORM_SCOPE_CODE
            })
            .map(|definition| definition.key.definition_code.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for seed in &seeds {
            if existing_codes.contains(seed.code) {
                continue;
            }
            let definition = seed_definition(seed, "v1-000", &current_timestamp());
            validate_seed_definition(&definition, self)?;
            self.repository(|repository| repository.insert_definition(&definition))?;
        }

        // The original first-delivery seeds used generic schemas. Repair only an exact
        // fingerprint of those generated snapshots; administrator changes always win.
        let existing = self.repository(|repository| repository.list_definitions())?;
        for seed in &seeds {
            let Some(legacy) = existing
                .iter()
                .find(|definition| is_known_legacy_seed(definition, seed))
            else {
                continue;
            };
            let replacement = seed_definition(
                seed,
                &next_definition_version(
                    &existing,
                    ResourceScopeKind::Platform,
                    PLATFORM_SCOPE_CODE,
                    seed.code,
                )?,
                &current_timestamp(),
            );
            validate_seed_definition(&replacement, self)?;
            self.repository(|repository| {
                repository.upgrade_builtin_definition(legacy, &replacement)
            })?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(super) struct BuiltinDefinition {
    pub(super) code: &'static str,
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    pub(super) attributes_schema: Value,
    pub(super) binary_schema: Value,
}

fn seed_definition(
    seed: &BuiltinDefinition,
    definition_version: &str,
    timestamp: &str,
) -> ResourceDefinition {
    ResourceDefinition {
        key: ResourceDefinitionKey {
            scope_kind: ResourceScopeKind::Platform,
            scope_code: PLATFORM_SCOPE_CODE.to_string(),
            definition_code: seed.code.to_string(),
            definition_version: definition_version.to_string(),
        },
        name: seed.name.to_string(),
        description: seed.description.to_string(),
        definition_status: ResourceDefinitionStatus::Enabled,
        allow_inheritance: true,
        attributes_schema: seed.attributes_schema.clone(),
        binary_schema: seed.binary_schema.clone(),
        binary_storage_profile: DEFAULT_BINARY_STORAGE_PROFILE.to_string(),
        draft_revision: 0,
        validation_status: ResourceValidationStatus::Valid,
        lifecycle_status: ResourceLifecycleStatus::Active,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
        published_at: Some(timestamp.to_string()),
    }
}

fn validate_seed_definition(
    definition: &ResourceDefinition,
    service: &ResourceService,
) -> Result<(), ResourceError> {
    if validate_definition_content(definition, &service.config.limits, &service.storage)?.is_empty()
    {
        Ok(())
    } else {
        Err(ResourceError::InvalidSchema)
    }
}

fn is_known_legacy_seed(definition: &ResourceDefinition, seed: &BuiltinDefinition) -> bool {
    legacy_builtin_definition(seed.code)
        .is_some_and(|legacy| is_generated_seed(definition, &legacy, &["v1-000"]))
        || previous_variable_password_seed(seed)
            .is_some_and(|previous| is_generated_seed(definition, &previous, &["v1-000", "v1-001"]))
}

fn is_generated_seed(
    definition: &ResourceDefinition,
    seed: &BuiltinDefinition,
    versions: &[&str],
) -> bool {
    definition.key.scope_kind == ResourceScopeKind::Platform
        && definition.key.scope_code == PLATFORM_SCOPE_CODE
        && definition.key.definition_code == seed.code
        && versions.contains(&definition.key.definition_version.as_str())
        && definition.name == seed.name
        && definition.description == seed.description
        && definition.definition_status == ResourceDefinitionStatus::Enabled
        && definition.allow_inheritance
        && definition.attributes_schema == seed.attributes_schema
        && definition.binary_schema == seed.binary_schema
        && definition.binary_storage_profile == DEFAULT_BINARY_STORAGE_PROFILE
        && definition.draft_revision == 0
        && definition.validation_status == ResourceValidationStatus::Valid
        && definition.lifecycle_status == ResourceLifecycleStatus::Active
}

fn builtin_definitions() -> Vec<BuiltinDefinition> {
    let mut definitions = vec![
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
            attributes_schema: image_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 10 }),
        },
        BuiltinDefinition {
            code: "PDF",
            name: "PDF",
            description: "A managed PDF resource.",
            attributes_schema: pdf_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 1 }),
        },
        BuiltinDefinition {
            code: "MYSQL_CONNECTION",
            name: "MySQL connection",
            description: "A managed connection configuration.",
            attributes_schema: mysql_schema(),
            binary_schema: no_binary_schema(),
        },
        BuiltinDefinition {
            code: "POSTGRESQL_CONNECTION",
            name: "PostgreSQL connection",
            description: "A managed connection configuration.",
            attributes_schema: postgresql_schema(),
            binary_schema: no_binary_schema(),
        },
        BuiltinDefinition {
            code: "REDIS_CONNECTION",
            name: "Redis connection",
            description: "A managed connection configuration.",
            attributes_schema: redis_schema(),
            binary_schema: no_binary_schema(),
        },
        BuiltinDefinition {
            code: "KAFKA_CONNECTION",
            name: "Kafka connection",
            description: "A managed connection configuration.",
            attributes_schema: kafka_schema(),
            binary_schema: no_binary_schema(),
        },
    ];
    definitions.extend(super::connection_seeds::server_builtin_definitions());
    definitions
}

fn legacy_builtin_definition(code: &str) -> Option<BuiltinDefinition> {
    match code {
        "FILE" => Some(BuiltinDefinition {
            code: "FILE",
            name: "File",
            description: "A managed file resource.",
            attributes_schema: legacy_file_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 1 }),
        }),
        "IMAGE" => Some(BuiltinDefinition {
            code: "IMAGE",
            name: "Image",
            description: "A managed image resource.",
            attributes_schema: legacy_file_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 10 }),
        }),
        "PDF" => Some(BuiltinDefinition {
            code: "PDF",
            name: "PDF",
            description: "A managed PDF resource.",
            attributes_schema: legacy_file_schema(),
            binary_schema: json!({ "minCount": 1, "maxCount": 1 }),
        }),
        "MYSQL_CONNECTION" => legacy_connection_definition("MYSQL_CONNECTION", "MySQL connection"),
        "POSTGRESQL_CONNECTION" => {
            legacy_connection_definition("POSTGRESQL_CONNECTION", "PostgreSQL connection")
        }
        "REDIS_CONNECTION" => legacy_connection_definition("REDIS_CONNECTION", "Redis connection"),
        "KAFKA_CONNECTION" => legacy_connection_definition("KAFKA_CONNECTION", "Kafka connection"),
        _ => None,
    }
}

fn no_binary_schema() -> Value {
    json!({ "minCount": 0, "maxCount": 0 })
}

fn file_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["fileName", "sizeBytes"],
        "properties": {
            "fileName": file_name_schema(),
            "fileExtension": file_extension_schema(),
            "sizeBytes": { "type": "integer", "minimum": 0 }
        },
        "additionalProperties": false
    })
}

fn image_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["images"],
        "properties": {
            "images": {
                "type": "array",
                "minItems": 1,
                "maxItems": 10,
                "items": {
                    "type": "object",
                    "required": ["binaryName", "fileName", "imageFormat", "sizeBytes"],
                    "properties": {
                        "binaryName": { "type": "string", "minLength": 1, "maxLength": 128 },
                        "fileName": file_name_schema(),
                        "fileExtension": file_extension_schema(),
                        "imageFormat": {
                            "type": "string",
                            "enum": ["JPEG", "PNG", "GIF", "WEBP", "SVG", "BMP", "AVIF"]
                        },
                        "sizeBytes": { "type": "integer", "minimum": 0 },
                        "width": { "type": "integer", "minimum": 1 },
                        "height": { "type": "integer", "minimum": 1 }
                    },
                    "additionalProperties": false
                }
            }
        },
        "additionalProperties": false
    })
}

fn pdf_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["fileName", "sizeBytes", "pageCount"],
        "properties": {
            "fileName": file_name_schema(),
            "sizeBytes": { "type": "integer", "minimum": 0 },
            "pageCount": { "type": "integer", "minimum": 1 },
            "title": { "type": "string", "minLength": 1, "maxLength": 512 },
            "author": { "type": "string", "minLength": 1, "maxLength": 255 },
            "pdfVersion": {
                "type": "string", "minLength": 1, "maxLength": 16,
                "pattern": "^[0-9]+\\.[0-9]+$"
            },
            "encrypted": { "type": "boolean" }
        },
        "additionalProperties": false
    })
}

fn mysql_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["connectionMode", "endpoints", "database", "credentials", "tls"],
        "properties": {
            "connectionMode": {
                "type": "string",
                "enum": ["SINGLE", "FAILOVER", "LOAD_BALANCE", "REPLICATION"]
            },
            "endpoints": {
                "type": "array", "minItems": 1, "uniqueItems": true,
                "items": {
                    "type": "object", "required": ["host", "port"],
                    "properties": {
                        "host": host_schema(),
                        "port": port_schema(),
                        "role": { "type": "string", "enum": ["PRIMARY", "REPLICA"] }
                    },
                    "additionalProperties": false
                }
            },
            "database": { "type": "string", "minLength": 1, "maxLength": 64 },
            "credentials": credentials_schema(true),
            "tls": tls_schema(
                &["DISABLED", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_IDENTITY"],
                false
            ),
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false,
        "allOf": [
            {
                "if": { "required": ["connectionMode"], "properties": { "connectionMode": { "const": "SINGLE" } } },
                "then": { "properties": { "endpoints": { "maxItems": 1 } } }
            },
            {
                "if": { "required": ["connectionMode"], "properties": { "connectionMode": { "const": "REPLICATION" } } },
                "then": {
                    "properties": {
                        "endpoints": {
                            "minItems": 2,
                            "items": { "required": ["role"] },
                            "allOf": [
                                { "contains": { "required": ["role"], "properties": { "role": { "const": "PRIMARY" } } } },
                                { "contains": { "required": ["role"], "properties": { "role": { "const": "REPLICA" } } } }
                            ]
                        }
                    }
                }
            },
            {
                "if": {
                    "required": ["connectionMode"],
                    "properties": { "connectionMode": { "not": { "const": "REPLICATION" } } }
                },
                "then": {
                    "properties": { "endpoints": { "items": { "not": { "required": ["role"] } } } }
                }
            }
        ]
    })
}

fn postgresql_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["endpoints", "database", "credentials", "targetSession", "loadBalance", "tls"],
        "properties": {
            "endpoints": {
                "type": "array", "minItems": 1, "uniqueItems": true,
                "items": endpoint_schema()
            },
            "database": { "type": "string", "minLength": 1, "maxLength": 63 },
            "credentials": credentials_schema(true),
            "targetSession": {
                "type": "string",
                "enum": ["ANY", "READ_WRITE", "READ_ONLY", "PRIMARY", "STANDBY", "PREFER_STANDBY"]
            },
            "loadBalance": { "type": "string", "enum": ["DISABLED", "RANDOM"] },
            "tls": tls_schema(
                &["DISABLED", "ALLOW", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"],
                true
            ),
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false
    })
}

fn redis_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["topology", "endpoints", "database", "tls"],
        "properties": {
            "topology": { "type": "string", "enum": ["STANDALONE", "SENTINEL", "CLUSTER"] },
            "endpoints": {
                "type": "array", "minItems": 1, "uniqueItems": true,
                "items": { "$ref": "#/definitions/endpoint" }
            },
            "database": { "type": "integer", "minimum": 0 },
            "credentials": { "$ref": "#/definitions/credentials" },
            "tls": { "$ref": "#/definitions/tls" },
            "sentinel": { "$ref": "#/definitions/sentinel" },
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false,
        "definitions": {
            "endpoint": endpoint_schema(),
            "credentials": credentials_schema(false),
            "tls": tls_schema(&["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true),
            "sentinel": {
                "type": "object", "required": ["masterName", "tls"],
                "properties": {
                    "masterName": {
                        "type": "string", "minLength": 1, "maxLength": 128, "pattern": "^\\S+$"
                    },
                    "credentials": { "$ref": "#/definitions/credentials" },
                    "tls": { "$ref": "#/definitions/tls" }
                },
                "additionalProperties": false
            }
        },
        "allOf": [
            {
                "if": { "required": ["topology"], "properties": { "topology": { "const": "STANDALONE" } } },
                "then": { "properties": { "endpoints": { "maxItems": 1 } } }
            },
            {
                "if": { "required": ["topology"], "properties": { "topology": { "const": "SENTINEL" } } },
                "then": { "required": ["sentinel"] }
            },
            {
                "if": {
                    "required": ["topology"],
                    "properties": { "topology": { "not": { "const": "SENTINEL" } } }
                },
                "then": { "not": { "required": ["sentinel"] } }
            },
            {
                "if": { "required": ["topology"], "properties": { "topology": { "const": "CLUSTER" } } },
                "then": { "properties": { "database": { "const": 0 } } }
            }
        ]
    })
}

fn kafka_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["brokers", "securityProtocol"],
        "properties": {
            "brokers": {
                "type": "array", "minItems": 1, "uniqueItems": true,
                "items": { "$ref": "#/definitions/endpoint" }
            },
            "securityProtocol": {
                "type": "string", "enum": ["PLAINTEXT", "SSL", "SASL_PLAINTEXT", "SASL_SSL"]
            },
            "sasl": { "$ref": "#/definitions/sasl" },
            "tls": { "$ref": "#/definitions/tls" },
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false,
        "definitions": {
            "endpoint": endpoint_schema(),
            "sasl": {
                "type": "object", "required": ["mechanism", "username", "password"],
                "properties": {
                    "mechanism": {
                        "type": "string", "enum": ["PLAIN", "SCRAM_SHA_256", "SCRAM_SHA_512"]
                    },
                    "username": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "password": secret_reference_schema()
                },
                "additionalProperties": false
            },
            "tls": tls_schema(&["REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true)
        },
        "allOf": [
            {
                "if": {
                    "required": ["securityProtocol"],
                    "properties": { "securityProtocol": { "enum": ["SASL_PLAINTEXT", "SASL_SSL"] } }
                },
                "then": { "required": ["sasl"] },
                "else": { "not": { "required": ["sasl"] } }
            },
            {
                "if": {
                    "required": ["securityProtocol"],
                    "properties": { "securityProtocol": { "enum": ["SSL", "SASL_SSL"] } }
                },
                "then": { "required": ["tls"] },
                "else": { "not": { "required": ["tls"] } }
            }
        ]
    })
}

fn file_name_schema() -> Value {
    json!({
        "type": "string", "minLength": 1, "maxLength": 255, "pattern": "^[^/\\\\]+$"
    })
}

fn file_extension_schema() -> Value {
    json!({
        "type": "string", "minLength": 1, "maxLength": 32,
        "description": "File extension without a leading dot."
    })
}

fn variable_reference_schema() -> Value {
    json!({ "type": "string", "format": "tron-variable-ref" })
}

fn credentials_schema(username_required: bool) -> Value {
    let required = if username_required {
        vec!["username", "password"]
    } else {
        vec!["password"]
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "username": { "type": "string", "minLength": 1, "maxLength": 128 },
            "password": secret_reference_schema()
        },
        "additionalProperties": false
    })
}

fn tls_schema(modes: &[&str], include_client_key_password: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "required": ["mode"],
        "properties": {
            "mode": { "type": "string", "enum": modes },
            "caCertificate": variable_reference_schema(),
            "clientCertificate": variable_reference_schema(),
            "clientKey": variable_reference_schema()
        },
        "additionalProperties": false
    });
    if include_client_key_password {
        schema["properties"]["clientKeyPassword"] = variable_reference_schema();
        schema["dependencies"] = json!({
            "clientCertificate": ["clientKey"],
            "clientKey": ["clientCertificate"],
            "clientKeyPassword": ["clientKey"]
        });
    }
    schema
}

fn scalar_properties_schema() -> Value {
    json!({
        "type": "object",
        "maxProperties": 128,
        "propertyNames": { "minLength": 1, "maxLength": 128 },
        "additionalProperties": { "type": ["string", "number", "boolean"] }
    })
}
