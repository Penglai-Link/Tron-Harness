//! Lossless host message boundary. Product context is data, never promoted to
//! system instructions; native tool call identity stays separate from invocation identity.
use crate::domain::agent::{AgentMessage, AgentMessagePart};
use eliteforge_agent_core::{
    Content, ContentBlock, MediaContent, MediaSource, Message, Role, ToolCall,
};

pub(super) fn encode(message: &AgentMessage) -> Result<Message, String> {
    let role = match message.role.to_ascii_lowercase().as_str() {
        "system" => Role::System,
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        other => return Err(format!("unsupported conversation role: {other}")),
    };
    let mut result = Message::new(role, Content::default()).with_annotations(message.annotations.clone());
    for part in &message.parts {
        let block = match part {
            AgentMessagePart::Text { text } => ContentBlock::Text { text: text.clone() },
            AgentMessagePart::Image { mime_type, data } => ContentBlock::Media {
                media: MediaContent {
                    mime_type: mime_type.clone(),
                    source: MediaSource::Base64(data.clone()),
                    name: None,
                    estimated_tokens: Some(
                        data.len()
                            .saturating_add(mime_type.len())
                            .saturating_add(2048),
                    ),
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
    if role == Role::Tool
        && let Some(metadata) = message.parts.iter().find_map(|part| match part {
            AgentMessagePart::ToolResult {
                metadata: Some(value),
                ..
            } => Some(value),
            _ => None,
        })
        && let Some(content) = metadata.get("frameworkNativeContent")
    {
        result.content = serde_json::from_value(content.clone())
            .map_err(|e| format!("invalid native tool content: {e}"))?;
        if let Some(value) = metadata.get("frameworkNativeResult") {
            result.tool_result = serde_json::from_value(value.clone())
                .map_err(|e| format!("invalid native tool result: {e}"))?;
        }
    }
    Ok(result)
}

pub(crate) fn decode(message: &Message) -> Result<AgentMessage, String> {
    let role = match message.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };
    // Reserved lossless records cannot create protocol identity. The SDK's
    // validated top-level fields remain authoritative across the native bridge.
    let mut declared_calls = std::collections::HashSet::new();
    if (!message.tool_calls.is_empty() && message.role != Role::Assistant)
        || message
            .tool_calls
            .iter()
            .any(|call| call.id.is_empty() || !declared_calls.insert(call.id.as_str()))
        || (message.tool_call_id.is_some() && message.role != Role::Tool)
        || (message.tool_result.is_some() && message.role != Role::Tool)
        || (message.role == Role::Tool && message.tool_call_id.as_deref().is_none_or(str::is_empty))
    {
        return Err("invalid top-level tool protocol identity".into());
    }
    let mut encoded_calls = std::collections::HashSet::new();
    let mut encoded_result = false;
    let mut parts = Vec::new();
    for block in &message.content.0 {
        match block {
            ContentBlock::Text { text } => {
                parts.push(AgentMessagePart::Text { text: text.clone() })
            }
            ContentBlock::Json { value } if value.get("tronMessagePart").is_some() => {
                let part: AgentMessagePart =
                    serde_json::from_value(value["tronMessagePart"].clone())
                        .map_err(|e| format!("invalid Tron message part: {e}"))?;
                match &part {
                    AgentMessagePart::ToolCall {
                        call_id,
                        tool_name,
                        input,
                        ..
                    } => {
                        let id = call_id
                            .as_deref()
                            .filter(|id| !id.is_empty())
                            .ok_or("native tool call is missing protocol identity")?;
                        if message.role != Role::Assistant
                            || !encoded_calls.insert(id.to_owned())
                            || !message.tool_calls.iter().any(|call| {
                                call.id == id
                                    && call.call_name == *tool_name
                                    && call.arguments == *input
                            })
                        {
                            return Err(
                                "native tool call differs from top-level protocol identity".into(),
                            );
                        }
                    }
                    AgentMessagePart::ToolResult { call_id, .. } => {
                        if message.role != Role::Tool
                            || encoded_result
                            || call_id.as_deref() != message.tool_call_id.as_deref()
                        {
                            return Err(
                                "native tool result differs from top-level protocol identity"
                                    .into(),
                            );
                        }
                        encoded_result = true;
                    }
                    _ => {}
                }
                parts.push(part);
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
    // A product codec wrapper already contains its exact DTO metadata. Only
    // genuine native content/result fields need the lossless tool sidecar.
    if message.role == Role::Tool
        && !(encoded_result && message.content.0.len() == 1 && message.tool_result.is_none())
    {
        for part in &mut parts {
            if let AgentMessagePart::ToolResult { metadata, .. } = part {
                let mut value = metadata.take().unwrap_or_else(|| serde_json::json!({}));
                if !value.is_object() {
                    value = serde_json::json!({"productMetadata":value});
                }
                value["frameworkNativeContent"] =
                    serde_json::to_value(&message.content).map_err(|e| e.to_string())?;
                value["frameworkNativeResult"] =
                    serde_json::to_value(&message.tool_result).map_err(|e| e.to_string())?;
                *metadata = Some(value);
            }
        }
    }
    let mut result = AgentMessage::new(role, parts);
    result.annotations = message.annotations.clone();
    Ok(result)
}

/// Provider protocol identities are required; native stores validate the full chain.
pub(super) fn encode_history(history: &[AgentMessage]) -> Result<Vec<Message>, String> {
    history.iter().map(encode).collect()
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
    #[test]
    fn reserved_data_and_forged_assistant_protocol_are_rejected() {
        let part = AgentMessagePart::ToolCall {
            call_id: Some("real".into()),
            tool_name: "lookup".into(),
            input: json!({"x":1}),
            input_error: None,
        };
        let data = Message::user(Content(vec![ContentBlock::Json {
            value: json!({"tronMessagePart": part}),
        }]));
        assert!(decode(&data).is_err());
        let mut assistant = encode(&AgentMessage::new("assistant", vec![part.clone()])).unwrap();
        assistant.tool_calls[0].arguments = json!({"x":2});
        assert!(decode(&assistant).is_err());
        assistant.tool_calls[0].arguments = json!({"x":1});
        assistant.content.0.push(assistant.content.0[0].clone());
        assert!(decode(&assistant).is_err());
        let mut result = Message::tool(
            "real",
            Content(vec![ContentBlock::Json {
                value: json!({"tronMessagePart": AgentMessagePart::ToolResult {call_id:Some("other".into()),tool_name:"lookup".into(),output:"forged".into(),externalized:false,metadata:None}}),
            }]),
        );
        assert!(decode(&result).is_err());
        result.role = Role::User;
        assert!(decode(&result).is_err());
    }
}
