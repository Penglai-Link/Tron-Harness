use serde_json::json;

use crate::domain::agent::{
    AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse,
    ModelStreamEvent, ModelTokenUsage,
};

use super::*;

#[test]
fn chat_response_accepts_content_array() {
    let response = ChatCompletionsAdapter
        .parse_response(&json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        { "type": "text", "text": "hello" },
                        { "type": "text", "text": " world" }
                    ]
                }
            }]
        }))
        .expect("content array should parse");

    assert_eq!(response, ModelResponse::final_output("hello world"));
}

#[test]
fn chat_response_accepts_legacy_function_call() {
    let response = ChatCompletionsAdapter
        .parse_response(&json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "function_call": {
                        "name": "lookup",
                        "arguments": "{\"query\":\"status\"}"
                    }
                }
            }]
        }))
        .expect("function_call should parse");

    assert_eq!(
        response,
        ModelResponse::tool_call("lookup", json!({ "query": "status" }))
    );
}

#[test]
fn chat_response_maps_openai_compatible_token_usage() {
    let response = ChatCompletionsAdapter
        .parse_response(&json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "hello"
                }
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        }))
        .expect("token usage response should parse");

    assert_eq!(
        response.token_usage(),
        Some(ModelTokenUsage {
            input_tokens: 11,
            output_tokens: 7,
            total_tokens: 18,
        })
    );
}

#[test]
fn chat_stream_usage_chunk_maps_openai_compatible_token_usage() {
    let usage = ChatCompletionsAdapter.parse_usage(&json!({
        "choices": [],
        "usage": {
            "prompt_tokens": 11,
            "completion_tokens": 7,
            "total_tokens": 18
        }
    }));

    assert_eq!(
        usage,
        Some(ModelTokenUsage {
            input_tokens: 11,
            output_tokens: 7,
            total_tokens: 18,
        })
    );
}

#[test]
fn chat_stream_request_asks_for_usage_chunk() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "gpt-test".into(),
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        timeout_ms: None,
        max_retries: None,
    };
    let body = ChatCompletionsAdapter
        .stream_request_body(
            &config,
            &ModelRequest {
                messages: vec![AgentMessage::user("hello")],
                tools: vec![],
            },
        )
        .expect("stream request should build");

    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["stream_options"]["include_usage"], json!(true));
}

#[test]
fn responses_response_skips_reasoning_and_reads_message_output_text() {
    let response = ResponsesAdapter
        .parse_response(&json!({
            "object": "response",
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "summary": [
                        { "type": "summary_text", "text": "thinking" }
                    ]
                },
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        { "type": "output_text", "text": "TRON" }
                    ]
                }
            ]
        }))
        .expect("message output after reasoning should parse");

    assert_eq!(response, ModelResponse::final_output("TRON"));
}

#[test]
fn responses_stream_completion_maps_nested_token_usage() {
    let usage = ResponsesAdapter.parse_usage(&json!({
        "type": "response.completed",
        "response": {
            "id": "resp_1",
            "usage": {
                "input_tokens": 13,
                "output_tokens": 8,
                "total_tokens": 21
            }
        }
    }));

    assert_eq!(
        usage,
        Some(ModelTokenUsage {
            input_tokens: 13,
            output_tokens: 8,
            total_tokens: 21,
        })
    );
}

#[test]
fn embeddings_usage_maps_openai_compatible_token_usage() {
    let usage = EmbeddingsAdapter.parse_usage(&json!({
        "usage": {
            "prompt_tokens": 5,
            "total_tokens": 5
        }
    }));

    assert_eq!(
        usage,
        Some(ModelTokenUsage {
            input_tokens: 5,
            output_tokens: 0,
            total_tokens: 5,
        })
    );
}

#[test]
fn responses_stream_maps_reasoning_and_output_deltas() {
    let reasoning = ResponsesAdapter
        .parse_stream_event(&json!({
            "type": "response.reasoning_text.delta",
            "delta": "Thinking Process:"
        }))
        .expect("reasoning stream event should parse safely");
    let output = ResponsesAdapter
        .parse_stream_event(&json!({
            "type": "response.output_text.delta",
            "delta": "final answer"
        }))
        .expect("output stream event should parse safely");
    let done = ResponsesAdapter
        .parse_stream_event(&json!({
            "type": "response.content_part.done",
            "part": {
                "content": [
                    { "type": "output_text", "text": "final answer" }
                ]
            }
        }))
        .expect("done stream event should parse safely");

    assert_eq!(
        reasoning,
        Some(ModelStreamEvent::reasoning_delta("Thinking Process:"))
    );
    assert_eq!(output, Some(ModelStreamEvent::text_delta("final answer")));
    assert_eq!(done, None);
}
