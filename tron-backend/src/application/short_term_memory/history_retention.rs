use crate::domain::short_term_memory::{ShortTermMemoryMessage, ShortTermMemoryPart};

use super::assembler::{estimate_messages_tokens, transcript_message_to_agent_message};

pub(super) type TranscriptEntry = (ShortTermMemoryMessage, Vec<ShortTermMemoryPart>);

#[derive(Debug, Clone)]
pub(super) struct HistoryRetentionSelection {
    pub(super) head_messages: Vec<TranscriptEntry>,
    pub(super) tail_messages: Vec<TranscriptEntry>,
    pub(super) source_start_seq: i64,
    pub(super) source_end_seq: i64,
    pub(super) tail_start_seq: i64,
    pub(super) pinned_start_seq: Option<i64>,
    pub(super) source_message_count: usize,
    pub(super) tail_token_estimate: u32,
}

/// Product rows map to SDK semantic metadata; selection belongs to memory.
pub(super) fn select_retained_history(
    messages: Vec<TranscriptEntry>,
    retained_token_limit: u32,
    pinned_run_code: Option<&str>,
    reasoning_summary_replay_enabled: bool,
) -> HistoryRetentionSelection {
    select(
        messages,
        retained_token_limit,
        pinned_run_code,
        None,
        reasoning_summary_replay_enabled,
    )
}
pub(super) fn select_history_from_tail_start(
    messages: Vec<TranscriptEntry>,
    tail_start_seq: i64,
    pinned_run_code: Option<&str>,
    reasoning_summary_replay_enabled: bool,
) -> HistoryRetentionSelection {
    select(
        messages,
        0,
        pinned_run_code,
        Some(tail_start_seq),
        reasoning_summary_replay_enabled,
    )
}
fn select(
    messages: Vec<TranscriptEntry>,
    tokens: u32,
    pinned_run: Option<&str>,
    requested: Option<i64>,
    reasoning: bool,
) -> HistoryRetentionSelection {
    let metadata = messages
        .iter()
        .map(|(message, parts)| {
            let ids = |kind: &str| {
                parts
                    .iter()
                    .filter(|p| p.part_type == kind && p.visibility == "model")
                    .filter_map(|p| {
                        p.content_json
                            .as_ref()?
                            .get("callId")?
                            .as_str()
                            .map(str::to_owned)
                    })
                    .collect()
            };
            let estimated_tokens =
                transcript_message_to_agent_message((message.clone(), parts.clone()), reasoning)
                    .map(|m| estimate_messages_tokens(&[m]))
                    .unwrap_or(0);
            eliteforge_agent_memory::HistoryEntry {
                sequence: message.seq,
                role: message.role.clone(),
                run_id: message.run_code.clone(),
                tool_calls: ids("tool_call"),
                tool_results: ids("tool_result"),
                estimated_tokens,
            }
        })
        .collect::<Vec<_>>();
    let selection =
        eliteforge_agent_memory::select_history(&metadata, tokens, pinned_run, requested);
    let source_start_seq = messages.first().map(|(m, _)| m.seq).unwrap_or(0);
    let source_end_seq = messages.last().map(|(m, _)| m.seq).unwrap_or(0);
    let tail_start_seq = messages
        .get(selection.tail_start)
        .map(|(m, _)| m.seq)
        .unwrap_or_else(|| source_end_seq.saturating_add(1));
    HistoryRetentionSelection {
        head_messages: messages[..selection.tail_start].to_vec(),
        tail_messages: messages[selection.tail_start..].to_vec(),
        source_start_seq,
        source_end_seq,
        tail_start_seq,
        pinned_start_seq: messages.get(selection.pinned_start).map(|(m, _)| m.seq),
        source_message_count: messages.len(),
        tail_token_estimate: selection.tail_tokens,
    }
}
