use serde_json::{Value, json};

use crate::domain::agent::AgentMessage;

pub(super) fn chat_message_json(message: &AgentMessage) -> Value {
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
        // Internal tool observations are serialized as plain user text because
        // provider chat APIs may reject `tool` messages unless they include a
        // provider-specific tool-call envelope and correlation id.
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
