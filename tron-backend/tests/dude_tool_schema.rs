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
        let tools: Vec<_> = BusinessToolCatalog::list_tools(None, None, locale)
            .into_iter().filter(|tool| tool.category.as_ref().is_some_and(|category| category.code == "tron.dude")).collect();
        assert_eq!(tools.len(), 16);
        for tool in tools {
            assert!(tool.tool_name.starts_with("dude_"));
            assert!(tool.domain_code.starts_with("dude_"));
            assert!(!tool.description.starts_with("agent.toolCatalog"));
            assert_eq!(tool.input_schema["type"], "object");
            assert_eq!(tool.input_schema["additionalProperties"], false);
            assert!(tool.input_schema.pointer("/properties/workspaceCode").is_none());
            if matches!(tool.domain_code.as_str(), "dude_work" | "dude_notification" | "dude_identity") {
                check(&tool.input_schema);
            }
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
fn workspace_scoped_catalog_tools_require_explicit_workspace() {
    let tools = BusinessToolCatalog::list_tools(None, None, "en-US");
    let mut workspace_tools = 0;
    for tool in tools {
        if tool.input_schema.pointer("/properties/workspaceCode").is_none() {
            continue;
        }
        workspace_tools += 1;
        assert!(tool.input_schema["required"].as_array().is_some_and(|required| required.iter().any(|key| key == "workspaceCode")), "{} must require workspaceCode", tool.tool_name);
        assert!(!tool.domain_code.starts_with("dude_"), "Dude tool {} must not require a workspace", tool.tool_name);
        for example in tool.input_schema["examples"].as_array().into_iter().flatten() {
            if example.is_object() {
                assert!(example["workspaceCode"].as_str().is_some_and(|code| !code.trim().is_empty()), "{} has an example without workspaceCode", tool.tool_name);
            }
        }
    }
    assert!(workspace_tools > 10, "workspace tool inventory unexpectedly empty");
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

#[test]
fn dude_task_contracts_keep_status_discovery_and_reopen_guidance() {
    for locale in ["en-US", "zh-CN"] {
        for code in ["dude.task_page", "dude.task_create"] {
            let description = BusinessToolCatalog::tool(code, locale).unwrap().description;
            assert!(!description.contains("workspace") && !description.contains("工作区"));
        }
        let get = BusinessToolCatalog::tool("dude.task_get", locale).unwrap();
        assert!(get.description.contains("taskFlow.statuses"));
        assert!(get.description.contains("dude_task_update"));
        let update = BusinessToolCatalog::tool("dude.task_update", locale).unwrap();
        let description = update.input_schema["properties"]["status"]["description"].as_str().unwrap();
        for status in ["NOT_STARTED", "IN_PROGRESS", "REOPEN", "CLOSED"] {
            assert!(description.contains(status), "{locale}: {description}");
        }
        assert!(description.contains("dude_task_get"));
        assert!(update.input_schema["properties"]["status"].get("enum").is_none(), "custom flows remain valid");
    }
}
