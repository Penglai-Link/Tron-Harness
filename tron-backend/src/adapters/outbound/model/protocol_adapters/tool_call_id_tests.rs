use serde_json::json;

use crate::domain::agent::{AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest};

use super::*;

#[test]
fn missing_tool_call_id_is_stabilized_for_responses_replay() {
    let body = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "function_call": {
                    "name": "lookup",
                    "arguments": "{\"query\":\"status\"}"
                }
            }
        }]
    });
    let response = ChatCompletionsAdapter
        .parse_response(&body)
        .expect("function_call should parse");
    let repeated_response = ChatCompletionsAdapter
        .parse_response(&body)
        .expect("function_call should parse repeatably");
    let call = response
        .tool_calls()
        .into_iter()
        .next()
        .expect("tool call should be present");
    let generated_call_id = call
        .call_id
        .expect("missing provider call id should be generated");
    assert!(generated_call_id.starts_with("generated_call_"));
    assert_eq!(
        repeated_response
            .tool_calls()
            .into_iter()
            .next()
            .and_then(|call| call.call_id),
        Some(generated_call_id.clone())
    );

    let config = |protocol| AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol,
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
    let request = ModelRequest {
        messages: vec![
            response.assistant.clone(),
            AgentMessage::tool_result("lookup", Some(generated_call_id.clone()), "found"),
        ],
        tools: vec![],
        system_agent_session_id: None,
        prepared_tool_schema: None,
        response_input_items: None,
        previous_response_id: None,
    };
    let chat = chat_completions_request(&config(AgentModelProtocol::ChatCompletions), &request);
    assert_eq!(
        chat["messages"][0]["tool_calls"][0]["id"],
        json!(generated_call_id)
    );
    assert_eq!(chat["messages"][1]["role"], json!("user"));
    assert!(chat["messages"][1]["tool_call_id"].is_null());
    assert_eq!(
        chat["messages"][1]["content"],
        json!("tool:lookup\nresult:\nfound")
    );

    let body = responses_request(&config(AgentModelProtocol::Responses), &request);
    let input = body["input"]
        .as_array()
        .expect("responses input should be array");
    assert_eq!(input[0]["type"], json!("function_call"));
    assert_eq!(input[0]["call_id"], json!(generated_call_id));
    assert_eq!(input[1]["type"], json!("function_call_output"));
    assert_eq!(input[1]["call_id"], input[0]["call_id"]);
    assert_eq!(input[1]["output"], json!("found"));
}

#[test]
fn responses_response_generates_stable_call_id_when_provider_omits_it() {
    let body = json!({
        "object": "response",
        "id": "resp_omitted_call_id",
        "status": "requires_action",
        "output": [
            {
                "type": "function_call",
                "name": "lookup",
                "arguments": "{\"query\":\"status\"}"
            }
        ]
    });
    let response = ResponsesAdapter
        .parse_response(&body)
        .expect("function call should parse without provider call id");
    let repeated_response = ResponsesAdapter
        .parse_response(&body)
        .expect("function call should parse repeatably");
    let generated_call_id = response
        .tool_calls()
        .into_iter()
        .next()
        .and_then(|call| call.call_id)
        .expect("missing provider call id should be generated");

    assert!(generated_call_id.starts_with("generated_call_"));
    assert_eq!(
        repeated_response
            .tool_calls()
            .into_iter()
            .next()
            .and_then(|call| call.call_id),
        Some(generated_call_id.clone())
    );

    let request = ModelRequest {
        messages: vec![
            response.assistant,
            AgentMessage::tool_result("lookup", Some(generated_call_id.clone()), "found"),
        ],
        tools: vec![],
        system_agent_session_id: None,
        prepared_tool_schema: None,
        response_input_items: None,
        previous_response_id: None,
    };
    let body = responses_request(
        &AgentModelConfig {
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
        },
        &request,
    );
    let input = body["input"]
        .as_array()
        .expect("responses input should be array");
    assert_eq!(input[0]["type"], json!("function_call"));
    assert_eq!(input[0]["call_id"], json!(generated_call_id));
    assert_eq!(input[1]["type"], json!("function_call_output"));
    assert_eq!(input[1]["call_id"], input[0]["call_id"]);
    assert_eq!(input[1]["output"], json!("found"));
}
