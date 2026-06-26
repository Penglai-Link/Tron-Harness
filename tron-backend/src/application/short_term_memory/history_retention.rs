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

pub(super) fn select_retained_history(
    messages: Vec<TranscriptEntry>,
    retained_token_limit: u32,
    _pinned_run_code: Option<&str>,
    reasoning_summary_replay_enabled: bool,
) -> HistoryRetentionSelection {
    let source_start_seq = messages
        .first()
        .map(|(message, _)| message.seq)
        .unwrap_or(0);
    let source_end_seq = messages.last().map(|(message, _)| message.seq).unwrap_or(0);
    let source_message_count = messages.len();
    if messages.is_empty() {
        return HistoryRetentionSelection {
            head_messages: Vec::new(),
            tail_messages: Vec::new(),
            source_start_seq,
            source_end_seq,
            tail_start_seq: source_end_seq.saturating_add(1),
            pinned_start_seq: None,
            source_message_count,
            tail_token_estimate: 0,
        };
    }

    let mut tail_start = minimum_replay_tail_start(&messages);
    let pinned_start_seq = messages.get(tail_start).map(|(message, _)| message.seq);

    let mut tail_token_estimate =
        estimate_history_tokens(&messages[tail_start..], reasoning_summary_replay_enabled);

    while tail_start > 0 {
        let candidate_start = previous_replay_boundary(&messages, tail_start);
        if candidate_start >= tail_start {
            break;
        }
        let candidate_tokens = estimate_history_tokens(
            &messages[candidate_start..],
            reasoning_summary_replay_enabled,
        );
        if candidate_tokens <= retained_token_limit {
            tail_start = candidate_start;
            tail_token_estimate = candidate_tokens;
            continue;
        }
        break;
    }

    let head_messages = messages[..tail_start].to_vec();
    let tail_messages = messages[tail_start..].to_vec();
    let tail_start_seq = tail_messages
        .first()
        .map(|(message, _)| message.seq)
        .unwrap_or_else(|| source_end_seq.saturating_add(1));

    HistoryRetentionSelection {
        head_messages,
        tail_messages,
        source_start_seq,
        source_end_seq,
        tail_start_seq,
        pinned_start_seq,
        source_message_count,
        tail_token_estimate,
    }
}

pub(super) fn select_history_from_tail_start(
    messages: Vec<TranscriptEntry>,
    tail_start_seq: i64,
    _pinned_run_code: Option<&str>,
    reasoning_summary_replay_enabled: bool,
) -> HistoryRetentionSelection {
    let source_start_seq = messages
        .first()
        .map(|(message, _)| message.seq)
        .unwrap_or(0);
    let source_end_seq = messages.last().map(|(message, _)| message.seq).unwrap_or(0);
    let source_message_count = messages.len();
    let requested_tail_start = messages
        .iter()
        .position(|(message, _)| message.seq >= tail_start_seq)
        .unwrap_or(messages.len());
    let tail_start = replay_boundary_at_or_before(&messages, requested_tail_start);
    let pinned_start_seq = messages.get(tail_start).map(|(message, _)| message.seq);
    let head_messages = messages[..tail_start].to_vec();
    let tail_messages = messages[tail_start..].to_vec();
    let tail_token_estimate =
        estimate_history_tokens(&tail_messages, reasoning_summary_replay_enabled);

    HistoryRetentionSelection {
        head_messages,
        tail_messages,
        source_start_seq,
        source_end_seq,
        tail_start_seq,
        pinned_start_seq,
        source_message_count,
        tail_token_estimate,
    }
}

fn minimum_replay_tail_start(messages: &[TranscriptEntry]) -> usize {
    if messages.is_empty() {
        return 0;
    }
    replay_boundary_at_or_before(messages, messages.len().saturating_sub(1))
}

fn previous_replay_boundary(messages: &[TranscriptEntry], tail_start: usize) -> usize {
    if tail_start == 0 {
        return 0;
    }
    replay_boundary_at_or_before(messages, tail_start - 1)
}

fn replay_boundary_at_or_before(messages: &[TranscriptEntry], index: usize) -> usize {
    if messages.is_empty() {
        return 0;
    }
    let bounded_index = index.min(messages.len().saturating_sub(1));
    if is_tool_result_message(&messages[bounded_index]) {
        return nearest_tool_call_message_index(messages, bounded_index).unwrap_or(bounded_index);
    }
    bounded_index
}

fn nearest_tool_call_message_index(
    messages: &[TranscriptEntry],
    before_or_at: usize,
) -> Option<usize> {
    messages[..=before_or_at]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, entry)| is_tool_call_message(entry).then_some(index))
}

fn is_tool_call_message((message, parts): &TranscriptEntry) -> bool {
    message.role == "assistant" && parts.iter().any(|part| part.part_type == "tool_call")
}

fn is_tool_result_message((message, parts): &TranscriptEntry) -> bool {
    message.role == "tool" || parts.iter().any(|part| part.part_type == "tool_result")
}

fn estimate_history_tokens(
    messages: &[TranscriptEntry],
    reasoning_summary_replay_enabled: bool,
) -> u32 {
    let rendered = messages
        .iter()
        .cloned()
        .filter_map(|entry| {
            transcript_message_to_agent_message(entry, reasoning_summary_replay_enabled)
        })
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        0
    } else {
        estimate_messages_tokens(&rendered)
    }
}
