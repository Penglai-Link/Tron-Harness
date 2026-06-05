use serde_json::json;

use crate::domain::agent::{
    AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest, ModelStreamEvent,
};

use super::{ChatCompletionsAdapter, ModelProtocolAdapter};

#[test]
fn chat_stream_request_asks_for_usage_chunk_and_thinking() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "gpt-test".into(),
        thinking_enabled: true,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    };
    let body = ChatCompletionsAdapter
        .stream_request_body(
            &config,
            &ModelRequest {
                messages: vec![AgentMessage::user("hello")],
                tools: vec![],
                system_agent_session_id: None,
                response_input_items: None,
                previous_response_id: None,
            },
        )
        .expect("stream request should build");

    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["stream_options"]["include_usage"], json!(true));
    assert_eq!(body["enable_thinking"], json!(true));
    assert_eq!(body["thinking"]["type"], json!("enabled"));
}

#[test]
fn chat_stream_maps_glm_thinking_content_to_summary_delta() {
    let event = ChatCompletionsAdapter
        .parse_stream_event(&json!({
            "choices": [{
                "delta": {
                    "thinking_content": "glm thinking"
                }
            }]
        }))
        .expect("chat thinking stream event should parse safely");

    assert_eq!(
        event,
        Some(ModelStreamEvent::reasoning_summary_delta("glm thinking"))
    );
}

#[test]
fn chat_stream_uses_reasoning_when_provider_sends_empty_content_field() {
    let event = ChatCompletionsAdapter
        .parse_stream_event(&json!({
            "choices": [{
                "delta": {
                    "content": "",
                    "reasoning_content": "glm reasoning"
                }
            }]
        }))
        .expect("chat reasoning stream event should parse safely");

    assert_eq!(
        event,
        Some(ModelStreamEvent::reasoning_summary_delta(
            "glm reasoning"
        ))
    );
}
