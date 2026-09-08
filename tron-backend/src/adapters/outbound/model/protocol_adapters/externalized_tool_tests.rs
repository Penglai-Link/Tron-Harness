use serde_json::json;

use crate::domain::agent::{
    AgentMessage, AgentMessagePart, AgentModelConfig, AgentModelProtocol, ModelRequest,
};

use super::{chat_completions_request, responses_request};

#[test]
fn request_builders_send_externalized_tool_notice_to_providers() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "gpt-test".into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: None,
    };
    let notice = json!({
        "externalized": true,
        "rootConfigKey": "shortTermMemory.toolOutput.root",
        "relativePath": "TRON_CORE/SESSION-1/RUN-1/0001-shell.exec.txt",
        "sha256": "abc123",
        "bytes": 2048,
        "lines": 20,
    })
    .to_string();
    let request = ModelRequest {
        messages: vec![AgentMessage::new(
            "tool",
            vec![AgentMessagePart::ToolResult {
                call_id: Some("call-123".into()),
                tool_name: "shell.exec".into(),
                output: notice.clone(),
                externalized: true,
                metadata: Some(json!({
                    "relativePath": "TRON_CORE/SESSION-1/RUN-1/0001-shell.exec.txt",
                    "sha256": "abc123",
                    "bytes": 2048,
                    "lines": 20,
                })),
            }],
        )],
        tools: vec![],
        system_agent_session_id: None,
        prepared_tool_schema: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&config, &request);
    assert_eq!(chat["messages"][0]["content"], json!(notice));
    assert!(
        !chat["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("large raw output")
    );

    let responses = responses_request(
        &AgentModelConfig {
            protocol: AgentModelProtocol::Responses,
            model_name: "qwen-test".into(),
            ..config
        },
        &request,
    );
    assert_eq!(responses["input"][0]["output"], json!(notice));
}
