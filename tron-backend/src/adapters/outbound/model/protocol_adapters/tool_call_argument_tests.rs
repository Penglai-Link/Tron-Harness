use serde_json::json;

use crate::domain::agent::{ModelFinish, ToolCall};

use super::{ChatCompletionsAdapter, ModelProtocolAdapter, ResponsesAdapter};

#[test]
fn chat_response_marks_missing_function_arguments_for_model_recovery() {
    let response = ChatCompletionsAdapter
        .parse_response(&json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": { "name": "fs.write_text" }
                    }]
                }
            }]
        }))
        .expect("function call without arguments should still be represented");

    assert_eq!(response.tool_calls(), vec![invalid_arguments_tool_call()]);
}

#[test]
fn responses_response_marks_missing_function_arguments_for_model_recovery() {
    let response = ResponsesAdapter
        .parse_response(&json!({
            "object": "response",
            "output": [{
                "call_id": "call-1",
                "type": "function_call",
                "name": "fs.write_text"
            }]
        }))
        .expect("function call without arguments should still be represented");

    assert_eq!(response.finish, ModelFinish::ToolCalls);
    assert_eq!(response.tool_calls(), vec![invalid_arguments_tool_call()]);
}

fn invalid_arguments_tool_call() -> ToolCall {
    ToolCall {
        call_id: Some("call-1".into()),
        tool_name: "fs.write_text".into(),
        input: serde_json::Value::Null,
        input_error: Some("function arguments are missing or are not a JSON string".into()),
    }
}
