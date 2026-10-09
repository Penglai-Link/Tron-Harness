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
        let Some(tool) = tools.iter().find(|tool| tool.tool_name == *tool_name) else {
            continue;
        };
        normalize_value(input, &tool.input_schema);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent::BusinessToolCatalog;
    use serde_json::json;

    #[test]
    fn quoted_attachment_reference_becomes_valid_without_accepting_bad_fields() {
        let definition = BusinessToolCatalog::tool("word.read", "en-US").unwrap();
        let tool: EffectiveTool = serde_json::from_value(json!({
            "toolName":"word_read", "description":"read", "inputSchema":definition.input_schema,
            "requiredVariableKeys":[]
        }))
        .unwrap();
        let validator = jsonschema::validator_for(&tool.input_schema).unwrap();
        for (reference, valid) in [
            (
                json!({"kind":"session_attachment","attachmentCode":"ATTACH-1"}),
                true,
            ),
            (
                json!({"kind":"session_attachment","attachmentCode":"ATTACH-1","path":"/private"}),
                false,
            ),
            (json!({"kind":"session_attachment"}), false),
        ] {
            let mut response =
                ModelResponse::tool_call("word_read", json!({"fileRef":reference.to_string()}));
            normalize(&mut response, std::slice::from_ref(&tool));
            let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else {
                panic!("tool call missing")
            };
            assert_eq!(input["fileRef"], reference);
            assert_eq!(validator.is_valid(input), valid);
        }
    }

    #[test]
    fn schema_string_parameters_and_malformed_json_are_not_reinterpreted() {
        let mut input =
            json!({"text":"{\"kind\":\"user-data\"}","fileRef":"{invalid","unknown":"{}"});
        let original = input.clone();
        normalize_value(
            &mut input,
            &json!({"type":"object","properties":{"text":{"type":"string"},"fileRef":{"type":"object"}}}),
        );
        assert_eq!(input, original);
    }
}
