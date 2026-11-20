//! Some compatible gateways return structured parameter values as JSON strings.
//! Restore only schema-declared objects/arrays before normal tool validation.
use crate::domain::agent::{AgentMessagePart, EffectiveTool, ModelResponse};
use serde_json::Value;

pub(super) fn normalize(response: &mut ModelResponse, tools: &[EffectiveTool]) {
    for part in &mut response.assistant.parts {
        let AgentMessagePart::ToolCall {
            tool_name,
            input,
            input_error,
            ..
        } = part
        else {
            continue;
        };
        if input_error.is_some() {
            continue;
        }
        // Normalize `tool_execute` envelopes in place: the inner arguments
        // follow the inner tool's schema, while the envelope shape stays
        // intact so gateway admission can still parse and unwrap it.
        if tool_name.as_str() == eliteforge_agent_core::TOOL_EXECUTE_CALL_NAME {
            if let Ok(envelope) = eliteforge_agent_core::GatewayEnvelope::parse(input) {
                let eliteforge_agent_core::GatewayEnvelope {
                    tool: name,
                    arguments,
                } = envelope;
                if let Some(tool) = tools.iter().find(|tool| tool.tool_name == name) {
                    let mut normalized = arguments;
                    normalize_arguments(&mut normalized, &tool.input_schema);
                    *input = serde_json::json!({
                        "tool": name,
                        "arguments": normalized,
                    });
                }
            }
            continue;
        }
        let Some(tool) = tools.iter().find(|tool| tool.tool_name == *tool_name) else {
            continue;
        };
        // Direct native calls and wrapped calls share the same schema-driven
        // normalization; no tool name or separate batch wrapper is involved.
        normalize_arguments(input, &tool.input_schema);
    }
}

/// Normalize one tool's arguments against its real schema: a batch schema is
/// consumed as-is, while an atomic schema projects the arguments' `items` array.
fn normalize_arguments(value: &mut Value, schema: &Value) {
    if schema.pointer("/properties/items").is_some() {
        normalize_value(value, schema);
    } else if let Some(items) = value.get_mut("items") {
        normalize_value(
            items,
            &serde_json::json!({"type": "array", "items": schema}),
        );
    } else {
        normalize_value(value, schema);
    }
}

fn normalize_value(value: &mut Value, schema: &Value) {
    if let Some(text) = value.as_str() {
        let expected = schema.get("type").and_then(Value::as_str);
        if matches!(expected, Some("object" | "array")) {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                if (expected == Some("object") && parsed.is_object())
                    || (expected == Some("array") && parsed.is_array())
                {
                    *value = parsed;
                }
            }
        }
    }
    if let (Some(object), Some(properties)) = (
        value.as_object_mut(),
        schema.get("properties").and_then(Value::as_object),
    ) {
        for (key, value) in object {
            if let Some(schema) = properties.get(key) {
                normalize_value(value, schema);
            }
        }
    } else if let (Some(items), Some(schema)) = (value.as_array_mut(), schema.get("items")) {
        for item in items {
            normalize_value(item, schema);
        }
    }
}
