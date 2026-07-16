use std::collections::BTreeSet;

use jsonschema::Draft;
use serde_json::{Map, Value, json};

use crate::{
    application::resource::ResourceValidationIssue,
    config::ResourceLimits,
    domain::{
        resource::{
            ResourceBinaryMapping, ResourceDefinition, ResourceError, validate_binary_name,
        },
        variable::validate_variable_key,
    },
};

use super::storage::StorageRouter;

#[derive(Debug, Clone)]
pub(super) struct BinaryPolicy {
    pub min_count: usize,
    pub max_count: usize,
    pub max_size_bytes: u64,
    pub name_pattern: Option<String>,
    pub max_name_length: usize,
}

pub(super) fn validate_definition_content(
    definition: &ResourceDefinition,
    limits: &ResourceLimits,
    storage: &StorageRouter,
) -> Result<Vec<ResourceValidationIssue>, ResourceError> {
    validate_definition_persistence(definition, limits)?;
    parse_binary_policy(&definition.binary_schema, limits)?;
    if !storage.has_profile(&definition.binary_storage_profile) {
        return Err(ResourceError::StorageProfileNotFound);
    }
    Ok(compile_schema(
        &definition.attributes_schema,
        limits.max_validation_errors,
    ))
}

pub(super) fn validate_definition_persistence(
    definition: &ResourceDefinition,
    limits: &ResourceLimits,
) -> Result<(), ResourceError> {
    if !definition.attributes_schema.is_object() || !definition.binary_schema.is_object() {
        return Err(ResourceError::InvalidInput);
    }
    ensure_json_limit(
        &definition.attributes_schema,
        limits.schema_max_bytes,
        limits.max_depth,
    )?;
    ensure_json_limit(
        &definition.binary_schema,
        limits.schema_max_bytes,
        limits.max_depth,
    )?;
    reject_external_references(&definition.attributes_schema)?;
    validate_variable_reference_schema(&definition.attributes_schema)
}

pub(super) fn parse_binary_policy(
    schema: &Value,
    limits: &ResourceLimits,
) -> Result<BinaryPolicy, ResourceError> {
    let object = schema
        .as_object()
        .ok_or(ResourceError::InvalidBinarySchema)?;
    if object.keys().any(|key| {
        !matches!(
            key.as_str(),
            "minCount" | "maxCount" | "maxSizeBytes" | "namePattern" | "maxNameLength"
        )
    }) {
        return Err(ResourceError::InvalidBinarySchema);
    }

    let min_count = optional_u64(object, "minCount")?.unwrap_or(0) as usize;
    let max_count = optional_u64(object, "maxCount")?.unwrap_or(0) as usize;
    let max_size_bytes = optional_u64(object, "maxSizeBytes")?.unwrap_or(limits.max_binary_bytes);
    let max_name_length = optional_u64(object, "maxNameLength")?
        .map(|value| value as usize)
        .unwrap_or(limits.max_binary_name_length);
    let name_pattern = object
        .get("namePattern")
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or(ResourceError::InvalidBinarySchema)
        })
        .transpose()?;

    if min_count > max_count
        || max_count > limits.max_binaries
        || max_size_bytes > limits.max_binary_bytes
        || max_name_length > limits.max_binary_name_length
    {
        return Err(ResourceError::InvalidBinarySchema);
    }
    if let Some(pattern) = &name_pattern {
        jsonschema::options()
            .with_draft(Draft::Draft7)
            .build(&json!({ "type": "string", "pattern": pattern }))
            .map_err(|_| ResourceError::InvalidBinarySchema)?;
    }

    Ok(BinaryPolicy {
        min_count,
        max_count,
        max_size_bytes,
        name_pattern,
        max_name_length,
    })
}

pub(super) fn validate_resource_draft(
    definition: &ResourceDefinition,
    attributes: &Value,
    mappings: &[ResourceBinaryMapping],
    limits: &ResourceLimits,
) -> Result<Vec<ResourceValidationIssue>, ResourceError> {
    if !attributes.is_object() {
        return Err(ResourceError::InvalidInput);
    }
    validate_definition_persistence(definition, limits)?;
    ensure_json_limit(attributes, limits.attributes_max_bytes, limits.max_depth)?;
    let mut errors = compile_instance(
        &definition.attributes_schema,
        attributes,
        limits.max_validation_errors,
    )?;
    let policy = parse_binary_policy(&definition.binary_schema, limits)?;
    errors.extend(validate_binary_set(&policy, mappings, limits));
    errors.truncate(limits.max_validation_errors);
    Ok(errors)
}

pub(super) fn validate_binary_upload(
    definition: &ResourceDefinition,
    binary_name: &str,
    byte_size: u64,
    existing: &[ResourceBinaryMapping],
    replacing: bool,
    limits: &ResourceLimits,
) -> Result<(), ResourceError> {
    let policy = parse_binary_policy(&definition.binary_schema, limits)?;
    validate_binary_name(binary_name, policy.max_name_length)?;
    if byte_size > policy.max_size_bytes || byte_size > limits.max_binary_bytes {
        return Err(ResourceError::InvalidInput);
    }
    if let Some(pattern) = policy.name_pattern {
        let validator = jsonschema::options()
            .with_draft(Draft::Draft7)
            .build(&json!({ "type": "string", "pattern": pattern }))
            .map_err(|_| ResourceError::InvalidBinarySchema)?;
        if !validator.is_valid(&json!(binary_name)) {
            return Err(ResourceError::InvalidInput);
        }
    }
    let projected_count = existing.len() + usize::from(!replacing);
    if projected_count > policy.max_count || projected_count > limits.max_binaries {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

pub(super) fn declared_variable_references(
    schema: &Value,
    attributes: &Value,
) -> Result<BTreeSet<String>, ResourceError> {
    let mut references = BTreeSet::new();
    collect_variable_references(schema, attributes, schema, 0, &mut references)?;
    Ok(references)
}

fn compile_schema(schema: &Value, max_errors: usize) -> Vec<ResourceValidationIssue> {
    match build_validator(schema) {
        Ok(_) => Vec::new(),
        Err(_) => vec![ResourceValidationIssue {
            path: "/".to_string(),
            message: "attributesSchema is not a valid Draft 7 schema".to_string(),
        }]
        .into_iter()
        .take(max_errors)
        .collect(),
    }
}

fn compile_instance(
    schema: &Value,
    attributes: &Value,
    max_errors: usize,
) -> Result<Vec<ResourceValidationIssue>, ResourceError> {
    let validator = build_validator(schema).map_err(|_| ResourceError::InvalidSchema)?;
    Ok(validator
        .iter_errors(attributes)
        .take(max_errors)
        .map(|error| ResourceValidationIssue {
            path: if error.instance_path().as_str().is_empty() {
                "/".to_string()
            } else {
                error.instance_path().as_str().to_string()
            },
            message: "attributes do not satisfy the pinned definition schema".to_string(),
        })
        .collect())
}

fn build_validator(schema: &Value) -> Result<jsonschema::Validator, ResourceError> {
    jsonschema::options()
        .with_draft(Draft::Draft7)
        .with_format("tron-variable-ref", |value| {
            validate_variable_key(value).is_ok()
        })
        .should_validate_formats(true)
        .build(schema)
        .map_err(|_| ResourceError::InvalidSchema)
}

fn validate_binary_set(
    policy: &BinaryPolicy,
    mappings: &[ResourceBinaryMapping],
    limits: &ResourceLimits,
) -> Vec<ResourceValidationIssue> {
    let mut errors = Vec::new();
    if mappings.len() < policy.min_count {
        errors.push(ResourceValidationIssue {
            path: "/binaries".to_string(),
            message: "binary count is below the definition minimum".to_string(),
        });
    }
    if mappings.len() > policy.max_count || mappings.len() > limits.max_binaries {
        errors.push(ResourceValidationIssue {
            path: "/binaries".to_string(),
            message: "binary count exceeds the definition limit".to_string(),
        });
    }
    let mut names = BTreeSet::new();
    let name_pattern_validator = policy.name_pattern.as_ref().and_then(|pattern| {
        jsonschema::options()
            .with_draft(Draft::Draft7)
            .build(&json!({ "type": "string", "pattern": pattern }))
            .ok()
    });
    for mapping in mappings {
        if validate_binary_name(&mapping.binary_name, policy.max_name_length).is_err()
            || name_pattern_validator
                .as_ref()
                .is_some_and(|validator| !validator.is_valid(&json!(mapping.binary_name)))
            || mapping.byte_size > policy.max_size_bytes
            || mapping.byte_size > limits.max_binary_bytes
            || !names.insert(mapping.binary_name.clone())
        {
            errors.push(ResourceValidationIssue {
                path: "/binaries".to_string(),
                message: "one or more binary mappings are invalid".to_string(),
            });
            break;
        }
    }
    errors
}

fn optional_u64(object: &Map<String, Value>, key: &str) -> Result<Option<u64>, ResourceError> {
    object
        .get(key)
        .map(|value| value.as_u64().ok_or(ResourceError::InvalidBinarySchema))
        .transpose()
}

fn ensure_json_limit(
    value: &Value,
    max_bytes: usize,
    max_depth: usize,
) -> Result<(), ResourceError> {
    if serde_json::to_vec(value)
        .map_err(|_| ResourceError::InvalidInput)?
        .len()
        > max_bytes
        || json_depth(value) > max_depth
    {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

fn json_depth(value: &Value) -> usize {
    match value {
        Value::Array(values) => 1 + values.iter().map(json_depth).max().unwrap_or(0),
        Value::Object(values) => 1 + values.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

fn reject_external_references(value: &Value) -> Result<(), ResourceError> {
    match value {
        Value::Array(values) => {
            for item in values {
                reject_external_references(item)?;
            }
        }
        Value::Object(values) => {
            if let Some(reference) = values.get("$ref") {
                if !reference
                    .as_str()
                    .is_some_and(|reference| reference == "#" || reference.starts_with("#/"))
                {
                    return Err(ResourceError::InvalidSchema);
                }
            }
            for item in values.values() {
                reject_external_references(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_variable_reference_schema(schema: &Value) -> Result<(), ResourceError> {
    match schema {
        Value::Array(values) => {
            for value in values {
                validate_variable_reference_schema(value)?;
            }
        }
        Value::Object(values) => {
            if values
                .get("format")
                .and_then(Value::as_str)
                .is_some_and(|format| format == "tron-variable-ref")
                && !declares_string(values)
            {
                return Err(ResourceError::InvalidSchema);
            }
            for value in values.values() {
                validate_variable_reference_schema(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn declares_string(schema: &Map<String, Value>) -> bool {
    match schema.get("type") {
        Some(Value::String(value)) => value == "string",
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some("string")),
        _ => false,
    }
}

fn collect_variable_references(
    schema: &Value,
    instance: &Value,
    root_schema: &Value,
    depth: usize,
    references: &mut BTreeSet<String>,
) -> Result<(), ResourceError> {
    if depth > 128 {
        return Err(ResourceError::InvalidSchema);
    }
    let Some(object) = schema.as_object() else {
        return Ok(());
    };
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        let target = local_reference(root_schema, reference).ok_or(ResourceError::InvalidSchema)?;
        return collect_variable_references(target, instance, root_schema, depth + 1, references);
    }
    if object
        .get("format")
        .and_then(Value::as_str)
        .is_some_and(|format| format == "tron-variable-ref")
    {
        let value = instance
            .as_str()
            .ok_or(ResourceError::InvalidVariableReference)?;
        validate_variable_key(value).map_err(|_| ResourceError::InvalidVariableReference)?;
        references.insert(value.trim().to_string());
    }
    if let (Some(properties), Some(values)) = (
        object.get("properties").and_then(Value::as_object),
        instance.as_object(),
    ) {
        for (key, property_schema) in properties {
            if let Some(value) = values.get(key) {
                collect_variable_references(
                    property_schema,
                    value,
                    root_schema,
                    depth + 1,
                    references,
                )?;
            }
        }
    }
    if let Some(values) = instance.as_array() {
        if let Some(items) = object.get("items") {
            match items {
                Value::Array(tuple) => {
                    for (index, value) in values.iter().enumerate() {
                        if let Some(item_schema) = tuple.get(index) {
                            collect_variable_references(
                                item_schema,
                                value,
                                root_schema,
                                depth + 1,
                                references,
                            )?;
                        }
                    }
                }
                _ => {
                    for value in values {
                        collect_variable_references(
                            items,
                            value,
                            root_schema,
                            depth + 1,
                            references,
                        )?;
                    }
                }
            }
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(items) = object.get(keyword).and_then(Value::as_array) {
            for item in items {
                collect_variable_references(item, instance, root_schema, depth + 1, references)?;
            }
        }
    }
    for keyword in ["if", "then", "else"] {
        if let Some(item) = object.get(keyword) {
            collect_variable_references(item, instance, root_schema, depth + 1, references)?;
        }
    }
    Ok(())
}

fn local_reference<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    if reference == "#" {
        Some(root)
    } else {
        root.pointer(reference.strip_prefix('#')?)
    }
}
