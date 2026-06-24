use serde_json::json;

use crate::domain::agent::ModelTokenUsage;

use super::{
    ChatCompletionsAdapter, EmbeddingsAdapter, ResponsesAdapter, registry::ModelProtocolAdapter,
};

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
                "total_tokens": 18,
                "prompt_tokens_details": {
                    "cached_tokens": 4
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 2
                }
            }
        }))
        .expect("token usage response should parse");

    assert_eq!(
        response.token_usage(),
        Some(ModelTokenUsage {
            input_tokens: 11,
            output_tokens: 7,
            total_tokens: 18,
            cached_input_tokens: Some(4),
            reasoning_tokens: Some(2),
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
            cached_input_tokens: None,
            reasoning_tokens: None,
        })
    );
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
                "total_tokens": 21,
                "input_tokens_details": {
                    "cached_tokens": 5
                },
                "output_tokens_details": {
                    "reasoning_tokens": 3
                }
            }
        }
    }));

    assert_eq!(
        usage,
        Some(ModelTokenUsage {
            input_tokens: 13,
            output_tokens: 8,
            total_tokens: 21,
            cached_input_tokens: Some(5),
            reasoning_tokens: Some(3),
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
            cached_input_tokens: None,
            reasoning_tokens: None,
        })
    );
}
