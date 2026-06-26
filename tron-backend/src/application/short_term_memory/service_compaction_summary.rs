use crate::{
    domain::short_term_memory::{ShortTermMemoryMessage, ShortTermMemoryPart},
    shared::time::current_timestamp,
};

const COMPACTION_RENDER_MAX_CHARS: usize = 2_000;

pub(super) fn compaction_transcript_summary(
    head_messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    tail_messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    reasoning_summary_replay_enabled: bool,
) -> (String, String) {
    let summary_lines = head_messages
        .iter()
        .flat_map(|(message, parts)| {
            let rendered = parts
                .iter()
                .filter_map(|part| {
                    render_compaction_part(message, part, reasoning_summary_replay_enabled)
                })
                .collect::<Vec<_>>();
            if rendered.is_empty() {
                vec![format!(
                    "[{} {}]: <no model-visible content>",
                    message.role, message.message_code
                )]
            } else {
                rendered
            }
        })
        .collect::<Vec<_>>();
    let summary = if summary_lines.is_empty() {
        format!(
            "No transcript content was available at {}.",
            current_timestamp()
        )
    } else {
        summary_lines.join("\n")
    };
    let recent = tail_messages
        .iter()
        .flat_map(|(message, parts)| {
            let rendered = parts
                .iter()
                .filter_map(|part| {
                    render_compaction_part(message, part, reasoning_summary_replay_enabled)
                })
                .collect::<Vec<_>>();
            if rendered.is_empty() {
                vec![format!(
                    "[{} {}]: <no model-visible content>",
                    message.role, message.message_code
                )]
            } else {
                rendered
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    (summary, recent)
}

fn render_compaction_part(
    message: &ShortTermMemoryMessage,
    part: &ShortTermMemoryPart,
    reasoning_summary_replay_enabled: bool,
) -> Option<String> {
    match part.part_type.as_str() {
        "text" => part.content_text.as_deref().map(|text| {
            format!(
                "[{} {}]: {}",
                message.role,
                message.message_code,
                truncate_chars(text, COMPACTION_RENDER_MAX_CHARS)
            )
        }),
        "reasoning" => None,
        "reasoning_summary" if reasoning_summary_replay_enabled => {
            part.content_text.as_deref().map(|text| {
                format!(
                    "[Assistant reasoning summary {}/{}]: {}",
                    message.message_code,
                    part.part_code,
                    truncate_chars(text, COMPACTION_RENDER_MAX_CHARS)
                )
            })
        }
        "reasoning_summary" => None,
        "tool_call" => {
            let content = part.content_json.as_ref()?;
            Some(format!(
                "[Tool call {}/{}]: {}({})",
                message.message_code,
                part.part_code,
                content
                    .get("toolName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                truncate_chars(
                    &content
                        .get("input")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "{}".into()),
                    COMPACTION_RENDER_MAX_CHARS,
                )
            ))
        }
        "tool_result" => {
            let content = part.content_json.as_ref();
            let tool_name = content
                .and_then(|value| value.get("toolName"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let externalized = content
                .and_then(|value| value.get("externalized"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            if externalized {
                let path = content
                    .and_then(|value| value.get("metadata"))
                    .and_then(|value| value.get("relativePath"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("externalized-output");
                Some(format!(
                    "[Tool result {}/{}]: {tool_name} output externalized at {path}",
                    message.message_code, part.part_code
                ))
            } else {
                Some(format!(
                    "[Tool result {}/{}]: {}",
                    message.message_code,
                    part.part_code,
                    truncate_chars(
                        part.content_text.as_deref().unwrap_or_default(),
                        COMPACTION_RENDER_MAX_CHARS
                    )
                ))
            }
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
            Some(format!("[Attached {mime}: {label}]"))
        }
        "compaction_summary" | "compaction_recent" => None,
        _ => part.content_text.as_deref().map(|text| {
            format!(
                "[{} {}/{}]: {}",
                part.part_type,
                message.message_code,
                part.part_code,
                truncate_chars(text, COMPACTION_RENDER_MAX_CHARS)
            )
        }),
    }
}

pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
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
