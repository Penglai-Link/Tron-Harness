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
                    if let Some(items) = normalized.get_mut("items") {
                        normalize_value(items, &serde_json::json!({
                            "type": "array", "items": tool.input_schema,
                        }));
                    } else {
                        normalize_value(&mut normalized, &tool.input_schema);
                    }
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

    fn gateway_pair() -> Vec<EffectiveTool> {
        vec![
            EffectiveTool {
                tool_name: "tool_search".into(),
                domain_code: None,
                description: "search".into(),
                input_schema: json!({"type":"object"}),
                skill_name: None,
                required_variable_keys: Vec::new(),
            },
            EffectiveTool {
                tool_name: "tool_execute".into(),
                domain_code: None,
                description: "execute".into(),
                input_schema: json!({"type":"object"}),
                skill_name: None,
                required_variable_keys: Vec::new(),
            },
            EffectiveTool {
                tool_name: "word_read".into(),
                domain_code: None,
                description: "read".into(),
                input_schema: json!({"type":"object","properties":{"fileRef":{"type":"object"}}}),
                skill_name: None,
                required_variable_keys: Vec::new(),
            },
        ]
    }

    #[test]
    fn gateway_envelope_arguments_are_normalized_against_the_inner_schema() {
        let tools = gateway_pair();
        let mut response = ModelResponse::tool_call(
            "tool_execute",
            json!({
                "tool": "word_read",
                "arguments": {
                    "fileRef": "{\"kind\":\"session_attachment\",\"attachmentCode\":\"ATTACH-1\"}"
                }
            }),
        );
        normalize(&mut response, &tools);
        let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else {
            panic!("tool call missing")
        };
        // The envelope shape survives; only the inner arguments are rewritten.
        assert_eq!(input["tool"], "word_read");
        assert_eq!(input["arguments"]["fileRef"]["attachmentCode"], "ATTACH-1");
    }

    #[test]
    fn gateway_envelope_input_alias_and_json_string_are_normalized() {
        let tools = gateway_pair();
        let mut response = ModelResponse::tool_call(
            "tool_execute",
            json!({"tool":"word_read","input":"{\"fileRef\":\"{\\\"kind\\\":\\\"session_attachment\\\"}\"}"}),
        );
        normalize(&mut response, &tools);
        let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else {
            panic!("tool call missing")
        };
        assert_eq!(input["tool"], "word_read");
        assert_eq!(input["arguments"]["fileRef"]["kind"], "session_attachment");
    }

    #[test]
    fn gateway_spread_null_and_invalid_arguments_follow_core_parser() {
        let tools = gateway_pair();
        for original in [
            json!({"tool":"word_read", "fileRef":"{\"kind\":\"session_attachment\"}"}),
            json!({"tool":"word_read", "arguments":null, "input":{"ignored":true}, "fileRef":"{\"kind\":\"session_attachment\"}"}),
        ] {
            let mut response = ModelResponse::tool_call("tool_execute", original);
            normalize(&mut response, &tools);
            let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else {
                panic!("missing call")
            };
            let envelope = eliteforge_agent_core::GatewayEnvelope::parse(input).unwrap();
            assert_eq!(
                envelope.arguments,
                json!({"fileRef":{"kind":"session_attachment"}})
            );
        }
        for raw in [json!([]), json!(false), json!("[]"), json!("not json")] {
            let original = json!({"tool":"word_read", "arguments":raw});
            let mut response = ModelResponse::tool_call("tool_execute", original.clone());
            normalize(&mut response, &tools);
            let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else {
                panic!("missing call")
            };
            assert_eq!(input, &original);
            assert!(eliteforge_agent_core::GatewayEnvelope::parse(input).is_err());
        }
    }

    #[test]
    fn gateway_batch_normalizes_each_item_without_inventing_a_single_item_wrapper() {
        let mut response = ModelResponse::tool_call("tool_execute", json!({
            "tool":"word_read", "arguments":{"items":"[{\"fileRef\":\"{\\\"kind\\\":\\\"session_attachment\\\"}\"},{\"fileRef\":\"broken\"}]"}
        }));
        normalize(&mut response, &gateway_pair());
        let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else { panic!("missing call") };
        assert_eq!(input["arguments"]["items"][0]["fileRef"]["kind"], "session_attachment");
        assert_eq!(input["arguments"]["items"][1]["fileRef"], "broken");
    }

    #[test]
    fn gateway_envelope_without_inner_tool_keeps_original_arguments() {
        let tools = gateway_pair();
        let original = json!({"tool":"unknown_tool","arguments":{"fileRef":"{}"}});
        let mut response = ModelResponse::tool_call("tool_execute", original.clone());
        normalize(&mut response, &tools);
        let AgentMessagePart::ToolCall { input, .. } = &response.assistant.parts[0] else {
            panic!("tool call missing")
        };
        assert_eq!(*input, original);
    }
}
