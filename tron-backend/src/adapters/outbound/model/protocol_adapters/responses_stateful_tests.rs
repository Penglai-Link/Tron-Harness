use serde_json::json;

use crate::domain::agent::{AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest};

use super::responses_request;

#[test]
fn responses_request_uses_previous_response_id_and_response_items_for_stateful_continuation() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        request_headers: Default::default(),
        model_name: "qwen-test".into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: true,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    };
    let body = responses_request(
        &config,
        &ModelRequest {
            messages: vec![
                AgentMessage::tool_with_call_id(
                    "tool:workspace.page\nresult:\n[]",
                    "call-123",
                    "[]",
                ),
            ],
            tools: vec![],
            system_agent_session_id: None,
            prepared_tool_schema: None,
            response_input_items: Some(vec![json!({
                "type": "function_call_output",
                "call_id": "call-123",
                "output": "[]"
            })]),
            previous_response_id: Some("resp_123".into()),
        },
    );

    assert_eq!(body["previous_response_id"], json!("resp_123"));
    assert_eq!(
        body["input"],
        json!([
            {
                "type": "function_call_output",
                "call_id": "call-123",
                "output": "[]"
            }
        ])
    );
}

#[test]
fn responses_stateful_rebase_preserves_tool_call_and_tool_result_items() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        request_headers: Default::default(),
        model_name: "qwen-test".into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: true,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    };
    let body = responses_request(
        &config,
        &ModelRequest {
            messages: vec![
                AgentMessage::system("You are helpful."),
                AgentMessage::user("Find the workspace."),
                AgentMessage::assistant_tool_calls(vec![(
                    Some("call-lookup".into()),
                    "workspace.lookup".into(),
                    json!({"workspaceCode": "TRON_CORE"}),
                )]),
                AgentMessage::tool_result(
                    "workspace.lookup",
                    Some("call-lookup".into()),
                    "{\"name\":\"Tron Core\"}",
                ),
            ],
            tools: vec![],
            system_agent_session_id: None,
            prepared_tool_schema: None,
            response_input_items: None,
            previous_response_id: None,
        },
    );

    let input = body["input"].as_array().expect("input should be an array");
    assert_eq!(input[0]["role"], json!("system"));
    assert_eq!(input[2]["type"], json!("function_call"));
    assert_eq!(input[2]["call_id"], json!("call-lookup"));
    assert_eq!(input[2]["name"], json!("workspace.lookup"));
    assert_eq!(input[3]["type"], json!("function_call_output"));
    assert_eq!(input[3]["call_id"], json!("call-lookup"));
    assert_eq!(input[3]["output"], json!("{\"name\":\"Tron Core\"}"));
    assert!(
        input.iter().all(|item| item["content"] != json!("")),
        "stateful replay must not emit empty assistant messages"
    );
}
