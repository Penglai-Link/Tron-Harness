use serde_json::json;
use tron_backend::{
    adapters::outbound::model::{chat_completions_request, responses_request},
    application::agent::{BusinessToolCatalog, tool_record_to_effective_tool},
    domain::agent::{AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest},
};

#[test]
fn work_change_schema_survives_resume_as_provider_compatible_object() {
    for locale in ["en-US", "zh-CN"] {
        let tool = tool_record_to_effective_tool(
            BusinessToolCatalog::tool("dude.work_change", locale).unwrap(),
        );
        let request = ModelRequest {
            messages: vec![AgentMessage::user("My name choice is Tron")],
            tools: vec![tool],
            prepared_tool_schema: None,
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        // A resumed request must expose the same concrete property definitions.
        let restored: ModelRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        for protocol in [
            AgentModelProtocol::Responses,
            AgentModelProtocol::ChatCompletions,
        ] {
            let config: AgentModelConfig = serde_json::from_value(json!({
                "protocol": protocol, "modelCode": "schema-test", "baseUrl": "http://127.0.0.1:1",
                "apiKey": "fixture", "modelName": "fixture"
            }))
            .unwrap();
            let payload = if protocol == AgentModelProtocol::Responses {
                responses_request(&config, &restored)
            } else {
                chat_completions_request(&config, &restored)
            };
            let descriptor = if protocol == AgentModelProtocol::Responses {
                &payload["tools"][0]
            } else {
                &payload["tools"][0]["function"]
            };
            assert_eq!(descriptor["name"], "dude_work_change");
            let schema = &descriptor["parameters"];
            assert_eq!(schema["type"], "object");
            assert!(schema["properties"].is_object());
            assert!(schema.get("oneOf").is_none());
            assert_eq!(schema["additionalProperties"], false);
            let validator = jsonschema::validator_for(schema).unwrap();
            for input in [
                json!({"operation":"list","taskCode":"task"}),
                json!({"operation":"recordResult","taskCode":"task","result":"Delivered"}),
                json!({"operation":"record","taskCode":"task","changeKey":"milestone","kind":"Progress","summary":"Built","evidence":["artifact:1"]}),
                json!({"operation":"resolve","changeId":"change","resolution":"User answered","evidence":["interaction:1"]}),
            ] {
                assert!(validator.is_valid(&input), "{locale}: {input}");
            }
            for input in [
                json!({"operation":"unknown"}),
                json!({"operation":"record","kind":"Wrong"}),
                json!({"operation":"record","evidence":[]}),
                json!({"operation":"list","unexpected":true}),
            ] {
                assert!(!validator.is_valid(&input), "{locale}: {input}");
            }
        }
    }
}

#[test]
fn all_dude_tools_have_closed_typed_provider_schemas_and_localized_descriptions() {
    fn check(schema: &serde_json::Value) {
        assert!(
            schema["type"].is_string(),
            "missing explicit type: {schema}"
        );
        if schema["type"] == "object" {
            assert_eq!(schema["additionalProperties"], false);
            let properties = schema["properties"].as_object().unwrap();
            for child in properties.values() {
                check(child);
            }
            if let Some(required) = schema["required"].as_array() {
                for key in required {
                    assert!(properties.contains_key(key.as_str().unwrap()));
                }
            }
        } else if schema["type"] == "array" {
            check(&schema["items"]);
        }
        for keyword in ["oneOf", "anyOf", "allOf", "$ref"] {
            assert!(
                schema.get(keyword).is_none(),
                "unsupported {keyword}: {schema}"
            );
        }
    }
    for locale in ["en-US", "zh-CN"] {
        let tools = BusinessToolCatalog::list_tools(Some("dude"), None, locale);
        assert_eq!(tools.len(), 4);
        for tool in tools {
            assert!(!tool.description.starts_with("agent.toolCatalog"));
            check(&tool.input_schema);
            let validator = jsonschema::validator_for(&tool.input_schema).unwrap();
            assert!(!validator.is_valid(&json!({"dudeCode":"another-dude"})));
            for example in tool.input_schema["examples"]
                .as_array()
                .into_iter()
                .flatten()
            {
                assert!(validator.is_valid(example), "{}: {example}", tool.tool_name);
            }
        }
    }
}

#[test]
fn soul_schema_rejects_invalid_shapes_and_describes_utf8_budget() {
    let schema = BusinessToolCatalog::tool("dude.soul_update", "en-US")
        .unwrap()
        .input_schema;
    let validator = jsonschema::validator_for(&schema).unwrap();
    for invalid in [
        json!({"name":""}),
        json!({"name":"Tron", "userRequest":"approved"}),
        json!({"name":"x".repeat(101)}),
        json!({"edits":[]}),
        json!({"edits":[{"new":"new"}]}),
        json!({"initialSoul":"x".repeat(16385)}),
        json!({"edits":[{"old":"x","new":"y","approved":true}]}),
    ] {
        assert!(!validator.is_valid(&invalid), "{invalid}");
    }
    assert!(validator.is_valid(&json!({"name":"川".repeat(100)})));
    assert!(
        schema["properties"]["initialSoul"]["description"]
            .as_str()
            .unwrap()
            .contains("UTF-8 bytes")
    );
}
