//! Lossless host message boundary. Product context is data, never promoted to
//! system instructions; native tool call identity stays separate from invocation identity.
use crate::domain::agent::{AgentMessage, AgentMessagePart};
use eliteforge_agent_core::{
    Content, ContentBlock, MediaContent, MediaSource, Message, Role, ToolCall,
};

pub(super) fn encode(message: &AgentMessage) -> Result<Message, String> {
    let role = match message.role.to_ascii_lowercase().as_str() {
        "system" => Role::System,
        "developer" => Role::Developer,
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        other => return Err(format!("unsupported conversation role: {other}")),
    };
    let mut result = Message::new(role, Content::default());
    for part in &message.parts {
        let block = match part {
            AgentMessagePart::Text { text } => ContentBlock::Text { text: text.clone() },
            AgentMessagePart::Image { mime_type, data } => ContentBlock::Media {
                media: MediaContent {
                    mime_type: mime_type.clone(),
                    source: MediaSource::Base64(data.clone()),
                    name: None,
                    estimated_tokens: None,
                },
            },
            AgentMessagePart::ToolCall {
                call_id,
                tool_name,
                input,
                ..
            } => {
                if role != Role::Assistant {
                    return Err("tool calls require assistant role".into());
                }
                let id = call_id
                    .as_ref()
                    .filter(|id| !id.is_empty())
                    .ok_or("tool call is missing provider identity")?;
                result
                    .tool_calls
                    .push(ToolCall::new(id, tool_name, input.clone()));
                ContentBlock::Json {
                    value: serde_json::json!({"tronMessagePart": part}),
                }
            }
            AgentMessagePart::ToolResult { call_id, .. } => {
                if role != Role::Tool || result.tool_call_id.is_some() {
                    return Err("one tool result per tool message is required".into());
                }
                result.tool_call_id = Some(
                    call_id
                        .as_ref()
                        .filter(|id| !id.is_empty())
                        .ok_or("tool result is missing provider identity")?
                        .clone(),
                );
                ContentBlock::Json {
                    value: serde_json::json!({"tronMessagePart": part}),
                }
            }
            _ => ContentBlock::Json {
                value: serde_json::json!({"tronMessagePart": part}),
            },
        };
        result.content.0.push(block);
    }
    Ok(result)
}

pub(super) fn decode(message: &Message) -> Result<AgentMessage, String> {
    let role = match message.role {
        Role::System => "system",
        Role::Developer => "developer",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };
    let mut parts = Vec::new();
    for block in &message.content.0 {
        match block {
            ContentBlock::Text { text } => {
                parts.push(AgentMessagePart::Text { text: text.clone() })
            }
            ContentBlock::Json { value } if value.get("tronMessagePart").is_some() => {
                parts.push(
                    serde_json::from_value(value["tronMessagePart"].clone())
                        .map_err(|e| format!("invalid Tron message part: {e}"))?,
                );
            }
            ContentBlock::Media { media } => {
                let data = match &media.source {
                    MediaSource::Base64(data) => data.clone(),
                    MediaSource::Url(url) => url.clone(),
                };
                parts.push(AgentMessagePart::Image {
                    mime_type: media.mime_type.clone(),
                    data,
                });
            }
            ContentBlock::Json { value } => parts.push(AgentMessagePart::Context {
                kind: "framework_json".into(),
                text: value.to_string(),
                metadata: Some(value.clone()),
            }),
            ContentBlock::Resource { resource } => parts.push(AgentMessagePart::Context {
                kind: "framework_resource".into(),
                text: resource.uri.clone(),
                metadata: Some(serde_json::to_value(resource).map_err(|e| e.to_string())?),
            }),
        }
    }
    for call in &message.tool_calls {
        if !parts.iter().any(|part| matches!(part, AgentMessagePart::ToolCall { call_id: Some(id), .. } if id == &call.id)) {
            parts.push(AgentMessagePart::ToolCall { call_id: Some(call.id.clone()), tool_name: call.call_name.clone(), input: call.arguments.clone(), input_error: None });
        }
    }
    if message.role == Role::Tool
        && !parts
            .iter()
            .any(|part| matches!(part, AgentMessagePart::ToolResult { .. }))
    {
        parts = vec![AgentMessagePart::ToolResult {
            call_id: message.tool_call_id.clone(),
            tool_name: "framework".into(),
            output: message.content.protocol_text(),
            externalized: message
                .tool_result
                .as_ref()
                .is_some_and(|r| r.result_reference.is_some()),
            metadata: message
                .tool_result
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|e| e.to_string())?,
        }];
    }
    Ok(AgentMessage::new(role, parts))
}

/// Legacy runs predate mandatory provider call IDs. Preserve their content while
/// pairing missing IDs deterministically; unmatched records remain labeled data.
pub(super) fn encode_history(history: &[AgentMessage]) -> Result<Vec<Message>, String> {
    use std::collections::VecDeque;
    let mut output = Vec::new();
    let mut pending = VecDeque::<(String, String)>::new();
    let close_pending = |pending: &mut VecDeque<(String, String)>, output: &mut Vec<Message>| {
        for (name, id) in pending.drain(..) {
            output.push(Message::tool(id, format!("Legacy history has no recorded result for {name}; execution outcome is unknown.")));
        }
    };
    for (sequence, original) in history.iter().enumerate() {
        let mut message = original.clone();
        if message.role != "tool" {
            close_pending(&mut pending, &mut output);
        }
        for (part_index, part) in message.parts.iter_mut().enumerate() {
            if let AgentMessagePart::ToolCall {
                call_id, tool_name, ..
            } = part
            {
                let id =
                    call_id.get_or_insert_with(|| format!("tron-legacy-{sequence}-{part_index}"));
                if id.is_empty() {
                    *id = format!("tron-legacy-{sequence}-{part_index}");
                }
                pending.push_back((tool_name.clone(), id.clone()));
            }
            if let AgentMessagePart::ToolResult {
                call_id, tool_name, ..
            } = part
            {
                let found = pending.iter().position(|(name, id)| {
                    if let Some(call_id) = call_id.as_ref().filter(|id| !id.is_empty()) {
                        id == call_id
                    } else {
                        name == tool_name
                    }
                });
                if let Some(index) = found {
                    *call_id = Some(pending.remove(index).unwrap().1);
                } else {
                    close_pending(&mut pending, &mut output);
                    output.push(Message::user(Content(vec![ContentBlock::Json {
                        value: serde_json::json!({"legacyUnpairedToolRecord":original}),
                    }])));
                    message.parts.clear();
                    break;
                }
            }
        }
        if !message.parts.is_empty() {
            output.push(encode(&message)?);
        }
    }
    close_pending(&mut pending, &mut output);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preserves_ordered_context_images_reasoning_and_checkpoints() {
        let original = AgentMessage::new(
            "user",
            vec![
                AgentMessagePart::Text {
                    text: "inspect this".into(),
                },
                AgentMessagePart::Context {
                    kind: "attachment".into(),
                    text: "attached file".into(),
                    metadata: Some(json!({"fileRef":{"resourceCode":"RESOURCE-1"}})),
                },
                AgentMessagePart::Image {
                    mime_type: "image/png".into(),
                    data: "aW1hZ2U=".into(),
                },
                AgentMessagePart::CompactionCheckpoint {
                    summary: "prior context".into(),
                    recent: "recent turns".into(),
                    source_start_seq: 1,
                    source_end_seq: 20,
                },
            ],
        );
        let encoded = encode(&original).unwrap();
        assert_eq!(encoded.role, Role::User);
        assert!(encoded.content.has_media());
        assert_eq!(decode(&encoded).unwrap(), original);
        let reasoning = AgentMessage::new(
            "assistant",
            vec![
                AgentMessagePart::Reasoning {
                    text: "private reasoning".into(),
                },
                AgentMessagePart::ReasoningSummary {
                    text: "public summary".into(),
                },
            ],
        );
        assert_eq!(decode(&encode(&reasoning).unwrap()).unwrap(), reasoning);
    }

    #[test]
    fn preserves_tool_errors_and_externalized_results_without_reassigning_call_ids() {
        let call = AgentMessage::new(
            "assistant",
            vec![AgentMessagePart::ToolCall {
                call_id: Some("provider-call".into()),
                tool_name: "fs_read_text".into(),
                input: json!({}),
                input_error: Some("malformed arguments".into()),
            }],
        );
        let encoded = encode(&call).unwrap();
        assert_eq!(encoded.tool_calls[0].id, "provider-call");
        assert!(!encoded.tool_calls[0].invocation_id.is_nil());
        assert_eq!(decode(&encoded).unwrap(), call);
        let result = AgentMessage::new(
            "tool",
            vec![AgentMessagePart::ToolResult {
                call_id: Some("provider-call".into()),
                tool_name: "fs_read_text".into(),
                output: "preview".into(),
                externalized: true,
                metadata: Some(json!({"resourceCode":"RESOURCE-2"})),
            }],
        );
        assert_eq!(decode(&encode(&result).unwrap()).unwrap(), result);
    }
}
