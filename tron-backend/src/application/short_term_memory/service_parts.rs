use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{
    application::short_term_memory::ToolTranscriptStatus,
    domain::{
        agent::{AgentMessagePart, ToolObservation},
        short_term_memory::ShortTermMemoryPart,
    },
};

pub(super) fn part_text(
    session_code: &str,
    workspace_code: &str,
    message_code: &str,
    seq: i64,
    part_type: &str,
    text: &str,
    visibility: &str,
) -> ShortTermMemoryPart {
    ShortTermMemoryPart {
        id: 0,
        part_code: String::new(),
        session_code: session_code.to_string(),
        workspace_code: workspace_code.to_string(),
        message_code: message_code.to_string(),
        seq,
        part_type: part_type.into(),
        status: "completed".into(),
        content_text: Some(text.to_string()),
        content_json: None,
        provider_metadata_json: None,
        visibility: visibility.into(),
        created_at: crate::shared::time::current_timestamp(),
        completed_at: Some(crate::shared::time::current_timestamp()),
    }
}

pub(super) fn tool_result_part(
    session_code: &str,
    workspace_code: &str,
    seq: i64,
    observation: &ToolObservation,
    call_id: Option<String>,
    call_index: u32,
    output: String,
    externalized: bool,
    metadata: Option<Value>,
    status: ToolTranscriptStatus,
) -> ShortTermMemoryPart {
    ShortTermMemoryPart {
        id: 0,
        part_code: String::new(),
        session_code: session_code.to_string(),
        workspace_code: workspace_code.to_string(),
        message_code: String::new(),
        seq,
        part_type: "tool_result".into(),
        status: status.part_status().into(),
        content_text: Some(output),
        content_json: Some(json!({
            "callId": call_id.clone(),
            "modelCallId": call_id,
            "callIndex": call_index,
            "toolName": observation.tool_name,
            "externalized": externalized,
            "metadata": metadata,
        })),
        provider_metadata_json: None,
        visibility: "model".into(),
        created_at: crate::shared::time::current_timestamp(),
        completed_at: Some(crate::shared::time::current_timestamp()),
    }
}

pub(super) fn transcript_part_from_agent_part(
    session_code: &str,
    workspace_code: &str,
    seq: i64,
    part: &AgentMessagePart,
    include_raw_reasoning: bool,
) -> Option<ShortTermMemoryPart> {
    match part {
        AgentMessagePart::Text { text } => Some(part_text(
            session_code,
            workspace_code,
            "",
            seq,
            "text",
            text,
            "model",
        )),
        AgentMessagePart::ReasoningSummary { text } => Some(part_text(
            session_code,
            workspace_code,
            "",
            seq,
            "reasoning_summary",
            text,
            "model",
        )),
        AgentMessagePart::Reasoning { text } if include_raw_reasoning => Some(part_text(
            session_code,
            workspace_code,
            "",
            seq,
            "reasoning",
            text,
            "audit",
        )),
        AgentMessagePart::ToolCall {
            call_id,
            tool_name,
            input,
            input_error,
        } => Some(ShortTermMemoryPart {
            id: 0,
            part_code: String::new(),
            session_code: session_code.to_string(),
            workspace_code: workspace_code.to_string(),
            message_code: String::new(),
            seq,
            part_type: "tool_call".into(),
            status: "completed".into(),
            content_text: None,
            content_json: Some(json!({
                "callId": call_id,
                "toolName": tool_name,
                "input": input,
                "inputError": input_error,
            })),
            provider_metadata_json: None,
            visibility: "model".into(),
            created_at: crate::shared::time::current_timestamp(),
            completed_at: Some(crate::shared::time::current_timestamp()),
        }),
        AgentMessagePart::Reasoning { .. }
        | AgentMessagePart::ToolResult { .. }
        | AgentMessagePart::Context { .. }
        | AgentMessagePart::Image { .. }
        | AgentMessagePart::CompactionCheckpoint { .. } => None,
    }
}

pub(super) fn estimate_text_tokens(text: &str) -> u32 {
    (text.len() as u32 / 4).max(1)
}

pub(super) fn expand_home(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return Path::new(&home).join(rest);
    }
    PathBuf::from(value)
}

pub(super) fn sanitize_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>();
    if trimmed.is_empty() {
        "tool".into()
    } else {
        trimmed
    }
}
