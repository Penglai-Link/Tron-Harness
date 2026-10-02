use crate::domain::short_term_memory::{ShortTermMemoryMessage, ShortTermMemoryPart};

/// Storage-to-memory mapping. Selection and summarization belong to the SDK.
/// Provider metadata and audit/internal parts never enter summary model input.
pub(super) fn authorized_compaction_history(
    messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    reasoning_summary_replay_enabled: bool,
) -> Vec<String> {
    messages.iter().map(|(message,parts)|serde_json::json!({
  "role":message.role,"messageCode":message.message_code,"sequence":message.seq,
  "parts":parts.iter().filter(|part|part.visibility=="model" && part.part_type!="reasoning" && (part.part_type!="reasoning_summary" || reasoning_summary_replay_enabled)).map(|part|serde_json::json!({"type":part.part_type,"partCode":part.part_code,"text":part.content_text,"data":part.content_json})).collect::<Vec<_>>()
 }).to_string()).collect()
}
pub(super) fn compaction_transcript_summary(
    head_messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    tail_messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    reasoning_summary_replay_enabled: bool,
) -> (String, String) {
    (
        authorized_compaction_history(head_messages, reasoning_summary_replay_enabled).join("\n"),
        authorized_compaction_history(tail_messages, reasoning_summary_replay_enabled).join("\n"),
    )
}
pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
    eliteforge_agent_memory::bound_text(value, max_chars)
}

pub(super) fn redact_todo_sections(value: &str) -> String {
    let mut retained = Vec::new();
    let mut skipping = false;
    for line in value.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            skipping = matches!(heading.trim(), "Current State" | "Completed Work");
        }
        if !skipping {
            retained.push(line);
        }
    }
    retained.join("\n").trim().to_string()
}
