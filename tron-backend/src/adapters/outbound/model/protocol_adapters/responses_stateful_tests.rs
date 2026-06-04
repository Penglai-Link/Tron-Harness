use serde_json::json;

use crate::domain::agent::{
    AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest,
};

use super::responses_request;

#[test]
fn responses_request_uses_previous_response_id_and_response_items_for_stateful_continuation() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "qwen-test".into(),
        thinking_enabled: false,
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
                AgentMessage::user("hello"),
                AgentMessage::tool_with_call_id(
                    "tool:workspace.list\nresult:\n[]",
                    "call-123",
                    "[]",
                ),
            ],
            tools: vec![],
            system_agent_session_id: None,
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
