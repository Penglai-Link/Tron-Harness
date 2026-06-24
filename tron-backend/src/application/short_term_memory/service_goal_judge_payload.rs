use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::domain::short_term_memory::{
    ShortTermMemoryGoalEvent, ShortTermMemoryGoalView, ShortTermMemoryInput,
    ShortTermMemoryMessage, ShortTermMemoryPart, ShortTermMemoryTodo,
};

const TOOL_SUMMARY_LIMIT: usize = 12;
const TOOL_RESULT_PREVIEW_CHARS: usize = 700;

#[allow(clippy::too_many_arguments)]
pub(super) fn goal_judge_payload(
    goal: &ShortTermMemoryGoalView,
    todos: &[ShortTermMemoryTodo],
    inputs: &[ShortTermMemoryInput],
    messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
    recent_events: &[ShortTermMemoryGoalEvent],
    run_code: &str,
    final_output: &str,
    final_output_hash: &str,
    step_count: u32,
) -> Value {
    json!({
        "instructions": {
            "task": "Judge whether the active session goal should continue.",
            "languagePolicy": "Use the evidence semantics. Do not rely on any single natural language completion keyword.",
            "continuationPolicy": "When verdict is CONTINUE, write a specific continuationPrompt for the next agent run. Do not repeat completed work.",
        },
        "goal": {
            "goalCode": goal.goal.goal_code,
            "objective": goal.goal.objective_markdown,
            "status": goal.goal.status,
            "runsUsed": goal.goal.runs_used,
            "runsUsedAfterJudge": goal.goal.runs_used.saturating_add(1),
            "maxRuns": goal.goal.max_runs,
            "lastVerdict": goal.goal.last_verdict,
            "lastReason": goal.goal.last_reason,
            "criteria": goal.criteria.iter()
                .filter(|criterion| criterion.status == "ACTIVE")
                .map(|criterion| json!({
                    "criterionCode": criterion.criterion_code,
                    "position": criterion.position,
                    "text": criterion.text,
                }))
                .collect::<Vec<_>>(),
        },
        "lastRun": {
            "runCode": run_code,
            "status": "COMPLETED",
            "stepCount": step_count,
            "finalOutput": final_output,
            "finalOutputHash": final_output_hash,
            "inputs": inputs.iter().map(input_summary).collect::<Vec<_>>(),
            "tools": tool_summaries(messages),
        },
        "workingState": {
            "activeTodos": todos.iter()
                .filter(|todo| matches!(todo.status.code(), "pending" | "in_progress"))
                .map(todo_summary)
                .collect::<Vec<_>>(),
            "completedTodoCount": todos.iter()
                .filter(|todo| todo.status.code() == "completed")
                .count(),
            "blockedTodos": Vec::<Value>::new(),
            "todoSnapshotHash": hash_text(&serde_json::to_string(todos).unwrap_or_default()),
        },
        "recentGoalEvents": recent_events.iter().map(goal_event_summary).collect::<Vec<_>>(),
    })
}

fn input_summary(input: &ShortTermMemoryInput) -> Value {
    json!({
        "inputCode": input.input_code,
        "kind": input.input_kind,
        "status": input.status,
        "prompt": input.prompt_json,
        "metadata": input.metadata_json,
    })
}

fn todo_summary(todo: &ShortTermMemoryTodo) -> Value {
    json!({
        "todoCode": todo.todo_code,
        "position": todo.position,
        "content": todo.content,
        "status": todo.status.code(),
        "priority": todo.priority.code(),
    })
}

fn tool_summaries(messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)]) -> Vec<Value> {
    messages
        .iter()
        .flat_map(|(_, parts)| parts.iter())
        .filter(|part| matches!(part.part_type.as_str(), "tool_call" | "tool_result"))
        .take(TOOL_SUMMARY_LIMIT)
        .map(|part| {
            let tool_name = part
                .content_json
                .as_ref()
                .and_then(|value| value.get("toolName"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            json!({
                "partCode": part.part_code,
                "type": part.part_type,
                "status": part.status,
                "toolName": tool_name,
                "externalized": part.content_json
                    .as_ref()
                    .and_then(|value| value.get("externalized"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "content": part.content_text
                    .as_deref()
                    .map(|text| truncate_chars(text, TOOL_RESULT_PREVIEW_CHARS)),
                "metadata": part.content_json,
            })
        })
        .collect()
}

fn goal_event_summary(event: &ShortTermMemoryGoalEvent) -> Value {
    json!({
        "eventSeq": event.event_seq,
        "eventKind": event.event_kind,
        "runCode": event.run_code,
        "summary": event.summary,
        "verdict": event.payload_json
            .as_ref()
            .and_then(|payload| payload.get("verdict"))
            .cloned(),
        "finalOutputHash": event_final_output_hash(event),
        "continuationPromptHash": event_continuation_prompt_hash(event),
    })
}

pub(super) fn no_progress_candidate(
    todos: &[ShortTermMemoryTodo],
    messages: &[(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)],
) -> bool {
    let active_todos_empty = todos
        .iter()
        .all(|todo| !matches!(todo.status.code(), "pending" | "in_progress"));
    let tool_parts_present = messages.iter().any(|(_, parts)| {
        parts
            .iter()
            .any(|part| matches!(part.part_type.as_str(), "tool_call" | "tool_result"))
    });
    active_todos_empty && !tool_parts_present
}

pub(super) fn no_progress_detected(
    candidate: bool,
    recent_events: &[ShortTermMemoryGoalEvent],
    final_output_hash: &str,
) -> bool {
    if !candidate {
        return false;
    }
    let recent_continue_events = recent_events
        .iter()
        .rev()
        .filter(|event| event_verdict(event) == Some("CONTINUE"))
        .take(2)
        .collect::<Vec<_>>();
    let repeated_final_output = recent_continue_events
        .iter()
        .any(|event| event_final_output_hash(event).as_deref() == Some(final_output_hash));
    repeated_final_output || recent_continue_events.len() >= 2
}

pub(super) fn no_progress_repeated_continuation_prompt(
    candidate: bool,
    continuation_prompt: Option<&str>,
    recent_events: &[ShortTermMemoryGoalEvent],
) -> bool {
    if !candidate {
        return false;
    }
    let Some(continuation_prompt_hash) = continuation_prompt.map(hash_text) else {
        return false;
    };
    recent_events
        .iter()
        .rev()
        .filter(|event| event_verdict(event) == Some("CONTINUE"))
        .take(2)
        .any(|event| {
            event_continuation_prompt_hash(event).as_deref()
                == Some(continuation_prompt_hash.as_str())
        })
}

fn event_verdict(event: &ShortTermMemoryGoalEvent) -> Option<&str> {
    event
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("verdict"))
        .and_then(Value::as_str)
}

fn event_final_output_hash(event: &ShortTermMemoryGoalEvent) -> Option<String> {
    event
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("finalOutputHash"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn event_continuation_prompt_hash(event: &ShortTermMemoryGoalEvent) -> Option<String> {
    event
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("continuationPromptHash"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            event
                .payload_json
                .as_ref()
                .and_then(|payload| payload.get("judgeOutput"))
                .and_then(|output| output.get("continuationPrompt"))
                .and_then(Value::as_str)
                .map(hash_text)
        })
}

pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut text = value
        .chars()
        .take(max_chars.saturating_sub(24))
        .collect::<String>();
    text.push_str("\n...[truncated]");
    text
}

pub(super) fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
