use serde_json::{Value, json};

use super::seeds::BuiltinDefinition;

const DRAFT_7: &str = "http://json-schema.org/draft-07/schema#";

pub(super) fn server_builtin_definitions() -> Vec<BuiltinDefinition> {
    vec![
        BuiltinDefinition {
            code: "SERVER_CONNECTION",
            name: "Server connection",
            description: "A managed SSH server connection.",
            attributes_schema: server_connection_schema(),
            binary_schema: no_binary_schema(),
        },
        BuiltinDefinition {
            code: "SERVER_ACCOUNT",
            name: "Server account",
            description: "A managed account that belongs to one server connection.",
            attributes_schema: server_account_schema(),
            binary_schema: no_binary_schema(),
        },
    ]
}

fn server_connection_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["host", "port", "protocol"],
        "properties": {
            "host": { "type": "string", "minLength": 1, "maxLength": 255 },
            "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
            "protocol": { "type": "string", "enum": ["Ssh"] }
        },
        "additionalProperties": false
    })
}

fn server_account_schema() -> Value {
    json!({
        "$schema": DRAFT_7,
        "type": "object",
        "required": ["serverCode", "username", "authenticationType"],
        "properties": {
            "serverCode": {
                "type": "string", "minLength": 1, "maxLength": 128,
                "pattern": "^[A-Za-z0-9][A-Za-z0-9._-]*$"
            },
            "username": { "type": "string", "minLength": 1, "maxLength": 128 },
            "authenticationType": {
                "type": "string", "enum": ["Password", "PrivateKey"]
            },
            "passwordVariableKey": variable_reference_schema(),
            "privateKeyVariableKey": variable_reference_schema(),
            "privateKeyPassphraseVariableKey": variable_reference_schema()
        },
        "additionalProperties": false,
        "allOf": [
            {
                "if": {
                    "required": ["authenticationType"],
                    "properties": { "authenticationType": { "const": "Password" } }
                },
                "then": {
                    "required": ["passwordVariableKey"],
                    "not": {
                        "anyOf": [
                            { "required": ["privateKeyVariableKey"] },
                            { "required": ["privateKeyPassphraseVariableKey"] }
                        ]
                    }
                }
            },
            {
                "if": {
                    "required": ["authenticationType"],
                    "properties": { "authenticationType": { "const": "PrivateKey" } }
                },
                "then": {
                    "required": ["privateKeyVariableKey"],
                    "not": { "required": ["passwordVariableKey"] }
                }
            }
        ]
    })
}

fn variable_reference_schema() -> Value {
    json!({ "type": "string", "format": "tron-variable-ref" })
}

fn no_binary_schema() -> Value {
    json!({ "minCount": 0, "maxCount": 0 })
}
