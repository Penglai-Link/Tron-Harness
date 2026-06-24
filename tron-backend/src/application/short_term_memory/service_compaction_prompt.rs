use sha2::{Digest, Sha256};

pub(super) struct CompactionPromptParts<'a> {
    pub previous_checkpoint: Option<&'a str>,
    pub history_to_compact: &'a str,
    pub retained_recent: &'a str,
    pub reason: Option<&'a str>,
    pub note: Option<&'a str>,
}

pub(super) fn render_compaction_prompt(parts: CompactionPromptParts<'_>) -> String {
    let previous_checkpoint = parts
        .previous_checkpoint
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("No previous checkpoint.");
    let history_to_compact = if parts.history_to_compact.trim().is_empty() {
        "No older transcript content is available for compaction."
    } else {
        parts.history_to_compact
    };
    let retained_recent = if parts.retained_recent.trim().is_empty() {
        "No retained recent context boundary."
    } else {
        parts.retained_recent
    };
    let reason = parts
        .reason
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unspecified");
    let note = parts
        .note
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("none");

    format!(
        r#"## Compaction Instructions
You are generating a short-term memory checkpoint for an agent conversation.
Return final text only. Do not call tools. Do not include raw chain-of-thought.

Preserve:
- active goal and user constraints
- current state and completed work
- durable decisions
- open questions or blockers
- concrete next steps
- critical facts
- tool results, files, entities, and identifiers needed to continue

Do not preserve:
- raw tool output beyond bounded evidence
- raw reasoning
- UI-only annotations
- content from outside the provided transcript

Compaction reason: {reason}
Operator note: {note}

## Previous Checkpoint
{previous_checkpoint}

## Serialized History To Compact
{history_to_compact}

## Retained Recent Context Boundary
The following recent context will be retained separately as raw bounded transcript.
Use it only to avoid losing continuity; do not duplicate it verbatim unless a fact must survive future compactions.

{retained_recent}

## Required Output Shape
Write Markdown with these headings:
## Goal
## User Constraints
## Current State
## Completed Work
## Decisions
## Open Questions / Blockers
## Next Steps
## Critical Facts
## Tools And Results
## Relevant Files / Entities
"#
    )
}

pub(super) fn hash_prompt(prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    hex::encode(hasher.finalize())
}
