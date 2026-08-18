use serde_json::{Value, json};

pub(super) fn endpoint_schema() -> Value {
    json!({
        "type": "object",
        "required": ["host", "port"],
        "properties": { "host": host_schema(), "port": port_schema() },
        "additionalProperties": false
    })
}

pub(super) fn host_schema() -> Value {
    json!({ "type": "string", "minLength": 1, "maxLength": 255 })
}

pub(super) fn port_schema() -> Value {
    json!({ "type": "integer", "minimum": 1, "maximum": 65535 })
}
