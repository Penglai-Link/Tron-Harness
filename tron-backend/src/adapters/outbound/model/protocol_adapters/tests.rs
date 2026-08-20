use serde_json::json;

use crate::domain::agent::{
    AgentMessage, AgentMessagePart, AgentModelConfig, AgentModelProtocol, ModelFinish,
    ModelRequest, ModelResponse, ModelStreamEvent, ToolCall,
};
use crate::domain::reasoning::ReasoningEffort;

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
fn chat_response_parses_multiple_tool_calls_without_parallel_flag_dependency() {
    let response = ChatCompletionsAdapter
        .parse_response(&json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "id": "call_123",
                            "type": "function",
                            "function": {
                                "name": "lookup",
                                "arguments": "{\"query\":\"status\"}"
                            }
                        },
                        {
                            "id": "call_456",
                            "type": "function",
                            "function": {
                                "name": "lookup",
                                "arguments": "{\"query\":\"health\"}"
                            }
                        }
                    ]
                }
            }]
        }))
        .expect("parallel tool call chat response should parse");

    assert_eq!(
        response,
        ModelResponse::from_tool_calls(vec![
            ToolCall {
                call_id: Some("call_123".into()),
                tool_name: "lookup".into(),
                input: json!({ "query": "status" }),
            },
            ToolCall {
                call_id: Some("call_456".into()),
                tool_name: "lookup".into(),
                input: json!({ "query": "health" }),
            },
        ])
    );
}

#[test]
fn request_builders_default_parallel_tool_calls_to_true_when_tools_are_present() {
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
    let request = ModelRequest {
        messages: vec![AgentMessage::user("hello")],
        tools: vec![crate::domain::agent::EffectiveTool {
            tool_name: "lookup".into(),
            domain_code: None,
            description: "Lookup something".into(),
            input_schema: json!({"type": "object"}),
            skill_name: None,
            required_variable_keys: vec![],
        }],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&config, &request);
    assert_eq!(chat["parallel_tool_calls"], json!(true));

    let responses = responses_request(
        &AgentModelConfig {
            protocol: AgentModelProtocol::Responses,
            model_name: "qwen-test".into(),
            responses_stateful_enabled: false,
            ..config
        },
        &request,
    );
    assert_eq!(responses["parallel_tool_calls"], json!(true));
}

#[test]
fn request_builders_preserve_nested_data_source_schema_and_examples() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "qwen-test".into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: None,
    };
    let tool = crate::application::agent::BusinessToolCatalog::tool(
        "resource.data_source.create",
        "en-US",
    )
    .expect("data source create tool should be catalogued");
    let request = ModelRequest {
        messages: vec![AgentMessage::user("create a mysql connection")],
        tools: vec![crate::domain::agent::EffectiveTool {
            tool_name: tool.tool_code,
            domain_code: Some(tool.domain_code),
            description: tool.description,
            input_schema: tool.input_schema.clone(),
            skill_name: None,
            required_variable_keys: vec![],
        }],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let responses = responses_request(&config, &request);
    assert_eq!(responses["tools"][0]["parameters"], tool.input_schema);
    assert!(
        responses["tools"][0]["parameters"]["examples"][0]["attributes"]["credentials"]["password"]
            ["source"]
            .is_string()
    );
    assert!(
        responses["tools"][0]["parameters"]["properties"]["attributes"]["oneOf"]
            .as_array()
            .is_some_and(|variants| variants.len() == 4)
    );

    let chat = chat_completions_request(
        &AgentModelConfig {
            protocol: AgentModelProtocol::ChatCompletions,
            ..config
        },
        &request,
    );
    assert_eq!(
        chat["tools"][0]["function"]["parameters"],
        tool.input_schema
    );
}

#[test]
fn request_builders_handle_tool_observations_per_protocol() {
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
    let request = ModelRequest {
        messages: vec![AgentMessage::tool_with_call_id(
            "tool:workspace.list\nresult:\n[]",
            "call-123",
            "[]",
        )],
        tools: vec![],
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&config, &request);
    assert_eq!(chat["messages"][0]["role"], json!("tool"));
    assert_eq!(chat["messages"][0]["tool_call_id"], json!("call-123"));
    assert_eq!(chat["messages"][0]["content"], json!("[]"));

    let responses = responses_request(&config, &request);
    assert_eq!(responses["input"][0]["type"], json!("function_call_output"));
    assert_eq!(responses["input"][0]["call_id"], json!("call-123"));
    assert_eq!(responses["input"][0]["output"], json!("[]"));
}

#[test]
fn responses_request_uses_string_input_for_single_user_prompt() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "qwen-test".into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
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
            ],
            tools: vec![],
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        },
    );

    assert_eq!(body["instructions"], json!("You are helpful."));
    assert_eq!(body["input"], json!("hello"));
}

#[test]
fn responses_request_uses_stateful_response_items_and_previous_response_id() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "qwen-test".into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    };
    let body = responses_request(
        &config,
        &ModelRequest {
            messages: vec![AgentMessage::tool_with_call_id(
                "tool:lookup\nresult:\nfound",
                "call-123",
                "found",
            )],
            tools: vec![],
            system_agent_session_id: None,
            response_input_items: Some(vec![json!({
                "type": "function_call_output",
                "call_id": "call-123",
                "output": "found"
            })]),
            previous_response_id: Some("resp-1".into()),
        },
    );

    assert_eq!(body["previous_response_id"], json!("resp-1"));
    assert_eq!(body["input"][0]["type"], json!("function_call_output"));
    assert_eq!(body["input"][0]["call_id"], json!("call-123"));
    assert_eq!(body["input"][0]["output"], json!("found"));
}

#[test]
fn chat_stream_maps_reasoning_content_to_raw_reasoning_delta() {
    let event = ChatCompletionsAdapter
        .parse_stream_event(&json!({
            "choices": [{
                "delta": {
                    "reasoning_content": "thinking detail"
                }
            }]
        }))
        .expect("chat reasoning stream event should parse safely");

    assert_eq!(
        event,
        Some(ModelStreamEvent::reasoning_delta("thinking detail"))
    );
}

#[test]
fn chat_stream_maps_reasoning_summary_content_to_summary_delta() {
    let event = ChatCompletionsAdapter
        .parse_stream_event(&json!({
            "choices": [{
                "delta": {
                    "reasoning_summary": "thinking summary"
                }
            }]
        }))
        .expect("chat reasoning summary stream event should parse safely");

    assert_eq!(
        event,
        Some(ModelStreamEvent::reasoning_summary_delta(
            "thinking summary"
        ))
    );
}

#[test]
fn responses_request_enables_reasoning_for_thinking_models() {
    let config = AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::Responses,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "qwen-test".into(),
        thinking_enabled: true,
        reasoning_effort: Some(ReasoningEffort::Medium),
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    };
    let body = ResponsesAdapter
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
        .expect("responses stream request should build");

    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["reasoning"]["effort"], json!("medium"));
    assert_eq!(body["reasoning"]["summary"], json!("concise"));
    assert!(body.get("enable_thinking").is_none());
}

#[test]
fn responses_response_reads_reasoning_summary_and_message_output_text() {
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

    assert_eq!(
        response,
        ModelResponse::from_assistant(
            AgentMessage::new(
                "assistant",
                vec![
                    AgentMessagePart::ReasoningSummary {
                        text: "thinking".into(),
                    },
                    AgentMessagePart::Text {
                        text: "TRON".into(),
                    },
                ],
            ),
            ModelFinish::Final,
            None,
            None,
            None,
        )
    );
}

#[test]
fn responses_response_parses_function_call_with_call_id() {
    let response = ResponsesAdapter
        .parse_response(&json!({
        "object": "response",
            "id": "resp_123",
            "status": "requires_action",
            "output": [
                {
                    "id": "fc_123",
                    "call_id": "call_123",
                    "type": "function_call",
                    "name": "lookup",
                    "arguments": "{\"query\":\"status\"}"
                }
            ]
        }))
        .expect("function call output should parse");

    assert_eq!(
        response,
        ModelResponse::tool_calls_with_usage_and_response_id(
            vec![ToolCall {
                call_id: Some("call_123".into()),
                tool_name: "lookup".into(),
                input: json!({ "query": "status" }),
            }],
            None,
            Some("resp_123".into()),
        )
    );
}

#[test]
fn responses_response_parses_multiple_function_calls() {
    let response = ResponsesAdapter
        .parse_response(&json!({
        "object": "response",
            "id": "resp_456",
            "status": "requires_action",
            "output": [
                {
                    "id": "fc_123",
                    "call_id": "call_123",
                    "type": "function_call",
                    "name": "lookup",
                    "arguments": "{\"query\":\"status\"}"
                },
                {
                    "id": "fc_456",
                    "call_id": "call_456",
                    "type": "function_call",
                    "name": "lookup",
                    "arguments": "{\"query\":\"health\"}"
                }
            ]
        }))
        .expect("multiple function call output should parse");

    assert_eq!(
        response,
        ModelResponse::tool_calls_with_usage_and_response_id(
            vec![
                ToolCall {
                    call_id: Some("call_123".into()),
                    tool_name: "lookup".into(),
                    input: json!({ "query": "status" }),
                },
                ToolCall {
                    call_id: Some("call_456".into()),
                    tool_name: "lookup".into(),
                    input: json!({ "query": "health" }),
                },
            ],
            None,
            Some("resp_456".into()),
        )
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
    let reasoning_summary = ResponsesAdapter
        .parse_stream_event(&json!({
            "type": "response.reasoning_summary_text.delta",
            "delta": "summary thinking"
        }))
        .expect("reasoning summary stream event should parse safely");
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
    assert_eq!(
        reasoning_summary,
        Some(ModelStreamEvent::reasoning_summary_delta(
            "summary thinking"
        ))
    );
    assert_eq!(output, Some(ModelStreamEvent::text_delta("final answer")));
    assert_eq!(done, None);
}
