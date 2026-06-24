use serde_json::{Value, json};

use crate::domain::agent::{
    AgentMessage, AgentMessagePart, AgentModelConfig, AgentModelProtocol, ModelRequest,
};

use super::{chat_completions_request, responses_request};

fn chat_config() -> AgentModelConfig {
    AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "gpt-test".into(),
        thinking_enabled: false,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: None,
    }
}

#[test]
fn request_builders_handle_tool_observations_per_protocol() {
    let config = chat_config();
    let request = ModelRequest {
        messages: vec![
            AgentMessage::assistant_tool_calls(vec![(
                Some("call-123".into()),
                "workspace.page".into(),
                json!({"workspaceCode": "TRON_CORE"}),
            )]),
            AgentMessage::tool_with_call_id("tool:workspace.page\nresult:\n[]", "call-123", "[]"),
        ],
        tools: vec![],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&config, &request);
    assert_eq!(chat["messages"][0]["role"], json!("assistant"));
    assert_eq!(chat["messages"][0]["content"], Value::Null);
    assert_eq!(
        chat["messages"][0]["tool_calls"][0]["id"],
        json!("call-123")
    );
    assert_eq!(
        chat["messages"][0]["tool_calls"][0]["function"]["name"],
        json!("workspace.page")
    );
    assert_eq!(chat["messages"][1]["role"], json!("tool"));
    assert_eq!(chat["messages"][1]["tool_call_id"], json!("call-123"));
    assert_eq!(chat["messages"][1]["content"], json!("[]"));

    let response_request = ModelRequest {
        messages: vec![AgentMessage::tool_with_call_id(
            "tool:workspace.page\nresult:\n[]",
            "call-123",
            "[]",
        )],
        tools: vec![],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };
    let responses = responses_request(&config, &response_request);
    assert_eq!(responses["input"][0]["type"], json!("function_call_output"));
    assert_eq!(responses["input"][0]["call_id"], json!("call-123"));
    assert_eq!(responses["input"][0]["output"], json!("[]"));
}

#[test]
fn chat_request_downgrades_tool_observations_without_call_id_to_user_text() {
    let request = ModelRequest {
        messages: vec![AgentMessage::new(
            "tool",
            vec![AgentMessagePart::Text {
                text: "tool:workspace.page\nresult:\n[]".into(),
            }],
        )],
        tools: vec![],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&chat_config(), &request);
    assert_eq!(chat["messages"][0]["role"], json!("user"));
    assert_eq!(
        chat["messages"][0]["content"],
        json!("tool:workspace.page\nresult:\n[]")
    );
}
