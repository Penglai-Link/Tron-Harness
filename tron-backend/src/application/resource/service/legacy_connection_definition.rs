use serde_json::{Value, json};

use super::seeds::BuiltinDefinition;

pub(super) fn legacy_connection_definition(
    code: &'static str,
    name: &'static str,
) -> Option<BuiltinDefinition> {
    Some(BuiltinDefinition {
        code,
        name,
        description: "A managed connection configuration.",
        attributes_schema: json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "host": { "type": "string", "minLength": 1 },
                "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
                "username": { "type": "string", "minLength": 1, "maxLength": 128 },
                "password": { "type": "string", "format": "tron-variable-ref" },
                "database": { "type": "string" }
            },
            "required": ["host", "port"],
            "additionalProperties": false
        }),
        binary_schema: json!({ "minCount": 0, "maxCount": 0 }),
    })
}

pub(super) fn legacy_file_schema() -> Value {
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
