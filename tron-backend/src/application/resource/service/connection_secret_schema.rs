use serde_json::{Value, json};

use super::seeds::BuiltinDefinition;

pub(super) fn secret_reference_schema() -> Value {
    json!({
        "oneOf": [
            variable_reference_schema(),
            {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "enum": ["DIRECT", "VARIABLE"] },
                    "value": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "variableKey": variable_reference_schema()
                },
                "additionalProperties": false,
                "oneOf": [
                    {
                        "required": ["source", "value"],
                        "properties": { "source": { "const": "DIRECT" } },
                        "not": { "required": ["variableKey"] }
                    },
                    {
                        "required": ["source", "variableKey"],
                        "properties": { "source": { "const": "VARIABLE" } },
                        "not": { "required": ["value"] }
                    }
                ]
            }
        ]
    })
}

pub(super) fn previous_variable_password_seed(
    seed: &BuiltinDefinition,
) -> Option<BuiltinDefinition> {
    let password_schema_path = match seed.code {
        "MYSQL_CONNECTION" | "POSTGRESQL_CONNECTION" => {
            "/properties/credentials/properties/password"
        }
        "REDIS_CONNECTION" => "/definitions/credentials/properties/password",
        "KAFKA_CONNECTION" => "/definitions/sasl/properties/password",
        _ => return None,
    };
    let mut previous = seed.clone();
    *previous
        .attributes_schema
        .pointer_mut(password_schema_path)? = variable_reference_schema();
    Some(previous)
}

fn variable_reference_schema() -> Value {
    json!({ "type": "string", "format": "tron-variable-ref" })
}
