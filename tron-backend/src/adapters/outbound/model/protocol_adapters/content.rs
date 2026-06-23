use serde_json::{Value, json};

use crate::domain::agent::AgentMessage;

pub(super) fn chat_message_json(message: &AgentMessage) -> Value {
    if message.role == "assistant"
        && let Some(tool_calls) = &message.tool_calls
    {
        return json!({
            "role": "assistant",
            "content": Value::Null,
            "tool_calls": tool_calls
                .iter()
                .map(|tool_call| json!({
                    "id": tool_call.call_id,
                    "type": "function",
                    "function": {
                        "name": tool_call.tool_name,
                        "arguments": serde_json::to_string(&tool_call.input)
                            .unwrap_or_else(|_| "{}".into()),
                    }
                }))
                .collect::<Vec<_>>(),
        });
    }
    if message.role == "tool"
        && let Some(tool_call_id) = &message.tool_call_id
    {
        return json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": message
                .tool_output
                .as_deref()
                .unwrap_or(message.content.as_str()),
        });
    }
    json!({
        "role": provider_role(message),
        "content": message.content,
    })
}

pub(super) fn responses_instructions_text(messages: &[AgentMessage]) -> Option<String> {
    let instructions = messages
        .iter()
        .filter(|message| matches!(message.role.as_str(), "system" | "developer"))
        .map(|message| message.content.trim())
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>();
    if instructions.is_empty() {
        None
    } else {
        Some(instructions.join("\n\n"))
    }
}

fn provider_role(message: &AgentMessage) -> &str {
    match message.role.as_str() {
        // Tool observations without provider call ids cannot be serialized as
        // native Chat Completions tool results, so keep the legacy plain-text
        // fallback for older function_call-style responses.
        "tool" => "user",
        other => other,
    }
}

pub(super) fn text_from_content(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return (!text.is_empty()).then(|| text.to_owned());
    }

    let text = content
        .as_array()?
        .iter()
        .filter_map(|item| {
            item.get("text")
                .or_else(|| item.get("content"))
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() { None } else { Some(text) }
}
