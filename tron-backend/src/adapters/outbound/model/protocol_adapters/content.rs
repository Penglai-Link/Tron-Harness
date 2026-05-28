use serde_json::{Value, json};

use crate::domain::agent::AgentMessage;

pub(super) fn chat_message_json(message: &AgentMessage) -> Value {
    json!({
        "role": message.role,
        "content": message.content,
    })
}

pub(super) fn response_message_json(message: &AgentMessage) -> Value {
    json!({
        "role": message.role,
        "content": message.content,
    })
}

pub(super) fn text_from_content(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
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
