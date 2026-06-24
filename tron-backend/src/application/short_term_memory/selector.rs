use serde_json::json;

use crate::domain::short_term_memory::{ShortTermMemoryMessage, ShortTermMemoryPart};

use super::CompactionPolicyConfig;

pub(super) type TranscriptEntry = (ShortTermMemoryMessage, Vec<ShortTermMemoryPart>);

const AGGRESSIVE_TOOL_RESULT_MAX_CHARS: usize = 256;
const SINGLE_TURN_TEXT_MAX_CHARS: usize = 512;
const SINGLE_TURN_REASONING_SUMMARY_MAX_CHARS: usize = 256;

#[derive(Debug, Clone)]
pub(super) struct RecentTranscriptSelector {
    max_recent_turns: usize,
    keep_recent_tokens: u32,
    tool_output_max_chars: usize,
    reasoning_inclusion: String,
    reasoning_summary_replay_enabled: bool,
}

#[derive(Debug, Clone)]
pub(super) struct RecentTranscriptSelection {
    pub(super) head_messages: Vec<TranscriptEntry>,
    pub(super) tail_messages: Vec<TranscriptEntry>,
    pub(super) source_start_seq: i64,
    pub(super) source_end_seq: i64,
    pub(super) tail_start_seq: i64,
    pub(super) source_message_count: usize,
    pub(super) tail_token_estimate: u32,
}

impl RecentTranscriptSelector {
    pub(super) fn new(
        policy: &CompactionPolicyConfig,
        keep_recent_tokens: u32,
        reasoning_summary_replay_enabled: bool,
    ) -> Self {
        Self {
            max_recent_turns: policy.max_recent_turns,
            keep_recent_tokens,
            tool_output_max_chars: policy.tool_output_max_chars.max(1),
            reasoning_inclusion: policy.reasoning_inclusion.clone(),
            reasoning_summary_replay_enabled,
        }
    }

    pub(super) fn select(&self, messages: Vec<TranscriptEntry>) -> RecentTranscriptSelection {
        let source_start_seq = messages
            .first()
            .map(|(message, _)| message.seq)
            .unwrap_or(0);
        let source_end_seq = messages.last().map(|(message, _)| message.seq).unwrap_or(0);
        let source_message_count = messages.len();

        if messages.is_empty() {
            return RecentTranscriptSelection {
                head_messages: Vec::new(),
                tail_messages: Vec::new(),
                source_start_seq,
                source_end_seq,
                tail_start_seq: source_end_seq,
                source_message_count,
                tail_token_estimate: 0,
            };
        }

        let initial_tail_start = self.initial_tail_start_index(&messages);
        let mut tail_start = initial_tail_start;
        let mut tail_source = messages[tail_start..].to_vec();
        let mut tail_messages = self.sanitize_messages(&tail_source, self.tool_output_max_chars);
        let mut tail_token_estimate = estimate_transcript_tokens(&tail_messages);

        if tail_token_estimate > self.keep_recent_tokens {
            let aggressive_cap = self
                .tool_output_max_chars
                .min(AGGRESSIVE_TOOL_RESULT_MAX_CHARS)
                .max(1);
            tail_messages = self.sanitize_messages(&tail_source, aggressive_cap);
            tail_token_estimate = estimate_transcript_tokens(&tail_messages);
        }

        while tail_token_estimate > self.keep_recent_tokens && contains_multiple_turns(&tail_source)
        {
            let drop_until = next_turn_start_index(&tail_source).unwrap_or(tail_source.len());
            tail_start += drop_until;
            tail_source = messages[tail_start..].to_vec();
            tail_messages = self.sanitize_messages(
                &tail_source,
                self.tool_output_max_chars
                    .min(AGGRESSIVE_TOOL_RESULT_MAX_CHARS)
                    .max(1),
            );
            tail_token_estimate = estimate_transcript_tokens(&tail_messages);
        }

        if tail_token_estimate > self.keep_recent_tokens && !tail_source.is_empty() {
            tail_messages =
                summarize_single_turn_messages(&tail_source, self.reasoning_summary_replay_enabled);
            tail_token_estimate = estimate_transcript_tokens(&tail_messages);
        }

        let head_messages = messages[..tail_start].to_vec();
        let tail_start_seq = tail_source
            .first()
            .map(|(message, _)| message.seq)
            .unwrap_or_else(|| source_end_seq.saturating_add(1));

        RecentTranscriptSelection {
            head_messages,
            tail_messages,
            source_start_seq,
            source_end_seq,
            tail_start_seq,
            source_message_count,
            tail_token_estimate,
        }
    }

    fn initial_tail_start_index(&self, messages: &[TranscriptEntry]) -> usize {
        if self.max_recent_turns == 0 {
            return messages.len();
        }
        let mut user_turns = 0usize;
        for index in (0..messages.len()).rev() {
            if is_user_turn(&messages[index].0) {
                user_turns += 1;
                if user_turns == self.max_recent_turns {
                    return index;
                }
            }
        }
        0
    }

    fn sanitize_messages(
        &self,
        messages: &[TranscriptEntry],
        tool_output_max_chars: usize,
    ) -> Vec<TranscriptEntry> {
        messages
            .iter()
            .filter_map(|(message, parts)| {
                let sanitized_parts = parts
                    .iter()
                    .filter_map(|part| {
                        sanitize_part(
                            part,
                            tool_output_max_chars,
                            self.reasoning_inclusion.as_str(),
                            self.reasoning_summary_replay_enabled,
                        )
                    })
                    .collect::<Vec<_>>();
                (!sanitized_parts.is_empty()).then(|| (message.clone(), sanitized_parts))
            })
            .collect()
    }
}

fn sanitize_part(
    part: &ShortTermMemoryPart,
    tool_output_max_chars: usize,
    reasoning_inclusion: &str,
    reasoning_summary_replay_enabled: bool,
) -> Option<ShortTermMemoryPart> {
    match part.part_type.as_str() {
        "text" | "tool_call" | "context" => Some(part.clone()),
        "reasoning_summary" if reasoning_summary_replay_enabled => Some(part.clone()),
        "reasoning_summary" => None,
        "reasoning" if reasoning_inclusion == "raw" => Some(part.clone()),
        "reasoning" => None,
        "tool_result" => {
            let mut next = part.clone();
            next.content_text = Some(truncate_chars(
                part.content_text.as_deref().unwrap_or_default(),
                tool_output_max_chars,
            ));
            Some(next)
        }
        "file" => {
            let label = part
                .content_json
                .as_ref()
                .and_then(|value| value.get("name").or_else(|| value.get("uri")))
                .and_then(|value| value.as_str())
                .unwrap_or("attachment");
            let mime = part
                .content_json
                .as_ref()
                .and_then(|value| value.get("mime"))
                .and_then(|value| value.as_str())
                .unwrap_or("file");
            let mut next = part.clone();
            next.part_type = "text".into();
            next.content_text = Some(format!("[Attached {mime}: {label}]"));
            next.content_json = Some(json!({
                "originalPartType": "file",
                "name": label,
                "mime": mime,
            }));
            Some(next)
        }
        _ => None,
    }
}

fn summarize_single_turn_messages(
    messages: &[TranscriptEntry],
    reasoning_summary_replay_enabled: bool,
) -> Vec<TranscriptEntry> {
    let pinned_user_seq = messages
        .iter()
        .rev()
        .find_map(|(message, _)| is_user_turn(message).then_some(message.seq));
    messages
        .iter()
        .filter_map(|(message, parts)| {
            let mut summary_parts = Vec::new();
            for part in parts {
                match part.part_type.as_str() {
                    "text" => {
                        let mut next = part.clone();
                        let text = part.content_text.as_deref().unwrap_or_default();
                        next.content_text = if Some(message.seq) == pinned_user_seq {
                            Some(text.to_string())
                        } else {
                            Some(truncate_chars(text, SINGLE_TURN_TEXT_MAX_CHARS))
                        };
                        summary_parts.push(next);
                    }
                    "reasoning_summary" if reasoning_summary_replay_enabled => {
                        let mut next = part.clone();
                        next.content_text = Some(truncate_chars(
                            part.content_text.as_deref().unwrap_or_default(),
                            SINGLE_TURN_REASONING_SUMMARY_MAX_CHARS,
                        ));
                        summary_parts.push(next);
                    }
                    "tool_call" => summary_parts.push(part.clone()),
                    "tool_result" => {
                        let mut next = part.clone();
                        let tool_name = part
                            .content_json
                            .as_ref()
                            .and_then(|value| value.get("toolName"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown");
                        next.content_text =
                            Some(format!("[Tool result omitted for budget: {tool_name}]"));
                        if let Some(content_json) = next.content_json.as_mut()
                            && let Some(object) = content_json.as_object_mut()
                        {
                            object.insert("omittedForBudget".into(), json!(true));
                        }
                        summary_parts.push(next);
                    }
                    "file" => {
                        let label = part
                            .content_json
                            .as_ref()
                            .and_then(|value| value.get("name").or_else(|| value.get("uri")))
                            .and_then(|value| value.as_str())
                            .unwrap_or("attachment");
                        let mut next = part.clone();
                        next.part_type = "text".into();
                        next.content_text = Some(format!("[Attached file omitted: {label}]"));
                        next.content_json = Some(json!({
                            "originalPartType": "file",
                            "name": label,
                        }));
                        summary_parts.push(next);
                    }
                    _ => {}
                }
            }
            (!summary_parts.is_empty()).then(|| (message.clone(), summary_parts))
        })
        .collect()
}

fn contains_multiple_turns(messages: &[TranscriptEntry]) -> bool {
    messages
        .iter()
        .filter(|(message, _)| is_user_turn(message))
        .nth(1)
        .is_some()
}

fn next_turn_start_index(messages: &[TranscriptEntry]) -> Option<usize> {
    messages
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, (message, _))| is_user_turn(message).then_some(index))
}

fn is_user_turn(message: &ShortTermMemoryMessage) -> bool {
    message.role == "user"
}

fn estimate_transcript_tokens(messages: &[TranscriptEntry]) -> u32 {
    messages
        .iter()
        .flat_map(|(_, parts)| parts.iter())
        .filter_map(part_text_for_estimate)
        .map(|text| estimate_text_tokens(&text))
        .sum()
}

fn part_text_for_estimate(part: &ShortTermMemoryPart) -> Option<String> {
    match part.part_type.as_str() {
        "text" | "reasoning_summary" | "context" => part.content_text.clone(),
        "reasoning" => None,
        "tool_call" => {
            let content = part.content_json.as_ref()?;
            Some(format!(
                "tool_call:{}\nargs:\n{}",
                content
                    .get("toolName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                content.get("input").cloned().unwrap_or_default()
            ))
        }
        "tool_result" => {
            let content = part.content_json.as_ref();
            let tool_name = content
                .and_then(|value| value.get("toolName"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let output = part.content_text.as_deref().unwrap_or_default();
            Some(format!("tool:{tool_name}\nresult:\n{output}"))
        }
        _ => None,
    }
}

fn estimate_text_tokens(text: &str) -> u32 {
    (text.len() as u32 / 4).max(1)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut text = value
        .chars()
        .take(max_chars.saturating_sub(24))
        .collect::<String>();
    text.push_str("\n...[content truncated]");
    text
}
