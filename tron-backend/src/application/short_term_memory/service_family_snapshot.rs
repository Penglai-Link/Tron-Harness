use crate::domain::short_term_memory::{ShortTermMemoryMessage, ShortTermMemoryPart};

pub(super) fn family_handoff_summary(
    messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    latest_checkpoint_summary: Option<&str>,
    run_code: &str,
    run_status: &str,
) -> String {
    let mut user_messages = 0usize;
    let mut assistant_messages = 0usize;
    let mut synthetic_messages = 0usize;
    let mut tool_messages = 0usize;
    let mut tool_results = 0usize;
    let mut externalized_tool_results = 0usize;
    let mut reasoning_parts = 0usize;
    let mut reasoning_summary_parts = 0usize;

    for (message, parts) in messages {
        match message.role.as_str() {
            "user" => user_messages += 1,
            "assistant" => assistant_messages += 1,
            "synthetic" => synthetic_messages += 1,
            "tool" => tool_messages += 1,
            _ => {}
        }
        for part in parts {
            match part.part_type.as_str() {
                "tool_result" => {
                    tool_results += 1;
                    if part
                        .content_json
                        .as_ref()
                        .and_then(|value| value.get("externalized"))
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                    {
                        externalized_tool_results += 1;
                    }
                }
                "reasoning" => reasoning_parts += 1,
                "reasoning_summary" => reasoning_summary_parts += 1,
                _ => {}
            }
        }
    }

    let source_start_seq = messages
        .first()
        .map(|(message, _)| message.seq)
        .unwrap_or_default();
    let source_end_seq = messages
        .last()
        .map(|(message, _)| message.seq)
        .unwrap_or_default();
    let mut sections = vec![
        format!(
            "## Handoff Summary\nRun {run_code} ended with status {run_status}. Raw transcript text, raw tool output, and raw reasoning are intentionally omitted from this cross-session snapshot."
        ),
        format!(
            "## Last Run Outcome\nRun code: {run_code}\nStatus: {run_status}\nSource transcript range: {source_start_seq}-{source_end_seq}"
        ),
        format!(
            "## Transcript Shape\n- user messages: {user_messages}\n- assistant messages: {assistant_messages}\n- synthetic messages: {synthetic_messages}\n- tool messages: {tool_messages}\n- tool results omitted: {tool_results}\n- externalized tool results referenced only by checkpoint metadata: {externalized_tool_results}\n- raw reasoning parts omitted: {reasoning_parts}\n- reasoning summary parts omitted from handoff snapshot: {reasoning_summary_parts}"
        ),
    ];
    if let Some(summary) = latest_checkpoint_summary.filter(|value| !value.trim().is_empty()) {
        sections.push(format!(
            "## Checkpoint Summary\n{}",
            truncate_chars(summary, 12_000)
        ));
    } else {
        sections.push(
            "## Checkpoint Summary\nNo completed checkpoint summary was available for this run handoff."
                .into(),
        );
    }
    sections.join("\n\n")
}

pub(super) fn checkpoint_summary_for_family(parts: &[ShortTermMemoryPart]) -> Option<String> {
    parts
        .iter()
        .find(|part| part.part_type == "compaction_summary")
        .and_then(|part| part.content_text.as_deref())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
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
