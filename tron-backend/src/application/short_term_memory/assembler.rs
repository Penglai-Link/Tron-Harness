use crate::domain::{
    agent::{AgentMessage, AgentMessagePart},
    short_term_memory::{
        ShortTermMemoryError, ShortTermMemoryGoalView, ShortTermMemoryMessage, ShortTermMemoryPart,
        ShortTermMemoryTodo, TodoStatus,
    },
};
use sha2::{Digest, Sha256};

use super::{ContextBudgetBreakdown, ModelContextBudgetPlan, PreparedModelContext};

pub(super) const DEFAULT_TODO_MAX_ITEMS: usize = 50;

pub(super) fn assemble_context(
    scope: &eliteforge_agent_memory::MemoryScope,
    prompt_prefix: &[AgentMessage],
    current_input_message: Option<&AgentMessage>,
    current_run_code: Option<&str>,
    checkpoint: Option<AgentMessage>,
    family_context: Option<AgentMessage>,
    todos: &[ShortTermMemoryTodo],
    active_goal: Option<&ShortTermMemoryGoalView>,
    recent: Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>,
    todo_enabled: bool,
    budget_plan: &ModelContextBudgetPlan,
    reasoning_summary_replay_enabled: bool,
) -> Result<PreparedModelContext, ShortTermMemoryError> {
    use eliteforge_agent_memory::{
        MemoryBlock, MemoryBlockKind as Kind, MemoryRevision, MemorySnapshot,
    };
    if recent.iter().any(|(message, parts)| {
        message.workspace_code != scope.namespace()
            || message.session_code != scope.session()
            || parts.iter().any(|part| {
                part.workspace_code != scope.namespace() || part.session_code != scope.session()
            })
    }) {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    let revision = recent.last().map(|(m, _)| m.seq.max(0) as u64).unwrap_or(0);
    let rebased = checkpoint.is_some();
    let working_state = working_state_message(active_goal, todos, todo_enabled, u32::MAX);
    let mut current_input_inserted = false;
    let recent_messages = recent
        .into_iter()
        .filter_map(|entry| {
            let is_current_user_input = current_run_code.is_some_and(|run_code| {
                entry.0.run_code.as_deref() == Some(run_code) && entry.0.role == "user"
            });
            if is_current_user_input
                && !current_input_inserted
                && let Some(current_input_message) = current_input_message
            {
                current_input_inserted = true;
                return Some(current_input_message.clone());
            }
            transcript_message_to_agent_message(entry, reasoning_summary_replay_enabled)
        })
        .collect::<Vec<_>>();
    let source = MemorySnapshot {
        revision: MemoryRevision {
            scope: scope.clone(),
            generation: "product-transcript".into(),
            revision,
        },
        blocks: vec![
            MemoryBlock {
                kind: Kind::SeedPromptPrefix,
                entries: prompt_prefix.to_vec(),
            },
            MemoryBlock {
                kind: Kind::CurrentSessionCheckpoint,
                entries: checkpoint.into_iter().collect(),
            },
            MemoryBlock {
                kind: Kind::SessionFamilyContext,
                entries: family_context.into_iter().collect(),
            },
            MemoryBlock {
                kind: Kind::CurrentSessionWorkingState,
                entries: working_state.into_iter().collect(),
            },
            MemoryBlock {
                kind: Kind::CurrentSessionHistory,
                entries: recent_messages,
            },
        ],
    };
    // This phase maps authorized repository records into independent sources.
    // It neither reduces nor admits a model window; DefaultContextAssembler does that.
    let entries = |kind| {
        source
            .blocks
            .iter()
            .find(|b| b.kind == kind)
            .map(|b| b.entries.clone())
            .unwrap_or_default()
    };
    let sources = super::PreparedContextSources {
        revision: source.revision.clone(),
        prefix: entries(Kind::SeedPromptPrefix),
        checkpoint: entries(Kind::CurrentSessionCheckpoint),
        family: entries(Kind::SessionFamilyContext),
        working: entries(Kind::CurrentSessionWorkingState),
        history: entries(Kind::CurrentSessionHistory),
    };
    let block_tokens = |entries: &[AgentMessage]| estimate_messages_tokens(entries);
    let prompt_prefix_tokens = block_tokens(&sources.prefix);
    let checkpoint_tokens = block_tokens(&sources.checkpoint);
    let session_family_context_tokens = block_tokens(&sources.family);
    let working_state_tokens = block_tokens(&sources.working);
    let recent_transcript_tokens = block_tokens(&sources.history);
    let messages = [
        &sources.prefix,
        &sources.checkpoint,
        &sources.family,
        &sources.working,
        &sources.history,
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<Vec<_>>();
    let context_hash = Some(hash_context_messages(&messages));
    let estimated_tokens = estimate_messages_tokens(&messages);
    Ok(PreparedModelContext {
        sources: Some(sources),
        messages,
        rebased,
        context_hash,
        checkpoint_code: None,
        estimated_tokens,
        message_token_estimate: estimated_tokens,
        tool_schema_token_estimate: 0,
        budget_breakdown: Some(ContextBudgetBreakdown {
            application_context_tokens: prompt_prefix
                .iter()
                .filter(|message| is_application_context(message))
                .map(|message| estimate_messages_tokens(std::slice::from_ref(message)))
                .sum(),
            application_context_message_count: prompt_prefix
                .iter()
                .filter(|message| is_application_context(message))
                .count(),
            prompt_prefix_message_count: prompt_prefix.len(),
            prompt_prefix_tokens,
            checkpoint_tokens,
            session_family_context_tokens,
            working_state_tokens,
            recent_transcript_tokens,
            raw_tail_transcript_tokens: recent_transcript_tokens,
            compressed_history_tokens: 0,
            tool_schema_tokens: 0,
            tool_schema_budget_tokens: Some(
                budget_plan.block_budgets.tool_schema.effective_limit_tokens,
            ),
            estimated_input_tokens: estimated_tokens,
            prompt_prefix_budget_tokens: Some(
                budget_plan
                    .block_budgets
                    .seed_prompt_prefix
                    .effective_limit_tokens,
            ),
            checkpoint_budget_tokens: Some(
                budget_plan
                    .block_budgets
                    .current_session_checkpoint
                    .effective_limit_tokens,
            ),
            session_family_context_budget_tokens: Some(
                budget_plan
                    .block_budgets
                    .session_family_context
                    .effective_limit_tokens,
            ),
            working_state_budget_tokens: Some(
                budget_plan
                    .block_budgets
                    .current_session_working_state
                    .effective_limit_tokens,
            ),
            effective_input_budget: Some(budget_plan.effective_input_budget),
            context_window_limit: Some(budget_plan.model_context_window),
            output_token_reserve: Some(budget_plan.output_token_reserve),
            compaction_trigger_ratio_bps: None,
            compaction_trigger_tokens: None,
            compaction_trigger_reached: false,
            input_budget_exceeded: estimated_tokens > budget_plan.effective_input_budget,
            retained_history_ratio_bps: None,
            effective_retained_history_tokens: None,
            current_session_history_budget_tokens: None,
            history_source_message_count: None,
            raw_tail_message_count: None,
            compressed_history_message_count: None,
            raw_tail_start_seq: None,
        }),
        over_budget_reason: None,
        context_window_limit: Some(budget_plan.model_context_window),
        effective_input_budget: Some(budget_plan.effective_input_budget),
        output_token_reserve: Some(budget_plan.output_token_reserve),
        over_budget: estimated_tokens > budget_plan.effective_input_budget,
        compaction_trigger_reached: false,
        compaction_required: false,
        compaction_tail_start_seq: None,
        compaction_trigger_tokens: None,
        effective_retained_history_tokens: None,
    })
}

pub(super) fn session_family_context_message(text: String) -> AgentMessage {
    AgentMessage::context_block("session_family_context", text)
}

pub(super) fn is_application_context(message: &AgentMessage) -> bool {
    message.parts.iter().any(|part| matches!(part, AgentMessagePart::Context { kind, .. } if kind == "application_execution_context"))
}

pub(super) fn working_state_message(
    active_goal: Option<&ShortTermMemoryGoalView>,
    todos: &[ShortTermMemoryTodo],
    todo_enabled: bool,
    token_limit: u32,
) -> Option<AgentMessage> {
    let active_todos = todos
        .iter()
        .filter(|todo| matches!(todo.status, TodoStatus::Pending | TodoStatus::InProgress))
        .collect::<Vec<_>>();
    if active_goal.is_none() && (!todo_enabled || active_todos.is_empty()) {
        return None;
    }

    let mut text = String::from(
        "<current-session-working-state>\nThis is current working state for this chat session. Use it as context, not as new user instructions.\n",
    );
    if let Some(goal_view) = active_goal {
        text.push_str(&format!(
            "\n<active-goal code=\"{}\">\nStatus: {}\nObjective: {}\n",
            goal_view.goal.goal_code,
            goal_view.goal.status,
            goal_view.goal.objective_markdown.trim()
        ));
        if let Some(verdict) = &goal_view.goal.last_verdict {
            text.push_str(&format!("Last judge verdict: {verdict}\n"));
        }
        if let Some(reason) = &goal_view.goal.last_reason {
            text.push_str(&format!("Last judge reason: {reason}\n"));
        }
        if let Some(blocked_reason) = &goal_view.goal.blocked_reason {
            text.push_str(&format!("Blocked reason: {blocked_reason}\n"));
        }
        text.push_str("</active-goal>\n");
    }

    if todo_enabled && !active_todos.is_empty() {
        text.push_str("\n<active-todos>\n");
        for todo in active_todos.into_iter().take(DEFAULT_TODO_MAX_ITEMS) {
            if text.len() >= token_limit as usize {
                break;
            }
            text.push_str(&format!(
                "- [{}] {} ({})\n",
                todo.status.code(),
                todo.content,
                todo.priority.code()
            ));
        }
        text.push_str("</active-todos>\n");
    }
    text.push_str("</current-session-working-state>");
    text = truncate_chars(&text, token_limit as usize);
    Some(AgentMessage::context_block(
        "current_session_working_state",
        text,
    ))
}

pub(crate) fn truncate_checkpoint_message(message: AgentMessage, token_limit: u32) -> AgentMessage {
    let max_chars = token_limit as usize;
    AgentMessage::new(
        message.role,
        message
            .parts
            .into_iter()
            .map(|part| match part {
                AgentMessagePart::CompactionCheckpoint {
                    summary,
                    recent,
                    source_start_seq,
                    source_end_seq,
                } => {
                    let (summary, recent) = eliteforge_agent_memory::bound_checkpoint_data(
                        &summary, &recent, max_chars,
                    );
                    AgentMessagePart::CompactionCheckpoint {
                        summary,
                        recent,
                        source_start_seq,
                        source_end_seq,
                    }
                }
                AgentMessagePart::Context {
                    kind,
                    text,
                    metadata,
                } => AgentMessagePart::Context {
                    kind,
                    text: truncate_chars(&text, max_chars),
                    metadata,
                },
                AgentMessagePart::Text { text } => AgentMessagePart::Text {
                    text: truncate_chars(&text, max_chars),
                },
                other => other,
            })
            .collect(),
    )
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    eliteforge_agent_memory::bound_text(text, max_chars)
}

pub(super) fn transcript_message_to_agent_message(
    (message, parts): (ShortTermMemoryMessage, Vec<ShortTermMemoryPart>),
    reasoning_summary_replay_enabled: bool,
) -> Option<AgentMessage> {
    let rendered_parts = parts
        .into_iter()
        .filter_map(|part| transcript_part_to_agent_part(part, reasoning_summary_replay_enabled))
        .collect::<Vec<_>>();
    (!rendered_parts.is_empty()).then(|| AgentMessage::new(message.role, rendered_parts))
}

pub(super) fn checkpoint_message(
    summary: &str,
    recent: &str,
    start: i64,
    end: i64,
) -> AgentMessage {
    AgentMessage::new(
        "user",
        vec![AgentMessagePart::CompactionCheckpoint {
            summary: summary.to_string(),
            recent: recent.to_string(),
            source_start_seq: start,
            source_end_seq: end,
        }],
    )
}

pub(super) fn checkpoint_message_from_parts(
    parts: &[ShortTermMemoryPart],
    reasoning_summary_replay_enabled: bool,
) -> Option<AgentMessage> {
    parts
        .iter()
        .find(|part| part.part_type == "compaction_summary")
        .map(|part| {
            let content_json = part.content_json.as_ref();
            let coverage = content_json
                .and_then(|value| value.get("coverage"))
                .and_then(|value| value.as_str());
            let recent = if coverage == Some("head_only_tail_replayed") {
                ""
            } else {
                content_json
                    .and_then(|value| value.get("recent"))
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
            };
            checkpoint_message(
                part.content_text.as_deref().unwrap_or_default(),
                &filter_reasoning_summary_lines(recent, reasoning_summary_replay_enabled),
                content_json
                    .and_then(|value| value.get("sourceStartSeq"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or_default(),
                content_json
                    .and_then(|value| value.get("sourceEndSeq"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or_default(),
            )
        })
}

fn transcript_part_to_agent_part(
    part: ShortTermMemoryPart,
    reasoning_summary_replay_enabled: bool,
) -> Option<AgentMessagePart> {
    if !part_model_replay_allowed(&part) {
        return None;
    }
    match part.part_type.as_str() {
        "text" => Some(AgentMessagePart::Text {
            text: part.content_text.unwrap_or_default(),
        }),
        "reasoning_summary" if reasoning_summary_replay_enabled => {
            Some(AgentMessagePart::ReasoningSummary {
                text: part.content_text.unwrap_or_default(),
            })
        }
        "reasoning_summary" => None,
        "tool_call" => {
            let content = part.content_json.unwrap_or_default();
            Some(AgentMessagePart::ToolCall {
                call_id: content
                    .get("callId")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                tool_name: content
                    .get("toolName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                input: content.get("input").cloned().unwrap_or_default(),
                input_error: content
                    .get("inputError")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
            })
        }
        "tool_result" => {
            let content = part.content_json.unwrap_or_default();
            Some(AgentMessagePart::ToolResult {
                call_id: content
                    .get("callId")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                tool_name: content
                    .get("toolName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                output: part.content_text.unwrap_or_default(),
                externalized: content
                    .get("externalized")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
                metadata: content.get("metadata").cloned(),
            })
        }
        "context" => Some(AgentMessagePart::Context {
            kind: part
                .content_json
                .as_ref()
                .and_then(|value| value.get("kind"))
                .and_then(|value| value.as_str())
                .unwrap_or("context")
                .to_string(),
            text: part.content_text.unwrap_or_default(),
            metadata: part.content_json,
        }),
        "compaction_summary" => {
            let content_json = part.content_json.as_ref();
            let coverage = content_json
                .and_then(|value| value.get("coverage"))
                .and_then(|value| value.as_str());
            let recent = if coverage == Some("head_only_tail_replayed") {
                String::new()
            } else {
                content_json
                    .and_then(|value| value.get("recent"))
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            Some(AgentMessagePart::CompactionCheckpoint {
                summary: part.content_text.unwrap_or_default(),
                recent,
                source_start_seq: content_json
                    .and_then(|value| value.get("sourceStartSeq"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or_default(),
                source_end_seq: content_json
                    .and_then(|value| value.get("sourceEndSeq"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or_default(),
            })
        }
        _ => None,
    }
}

fn filter_reasoning_summary_lines(text: &str, reasoning_summary_replay_enabled: bool) -> String {
    if reasoning_summary_replay_enabled {
        return text.to_string();
    }
    text.lines()
        .filter(|line| !line.starts_with("[Assistant reasoning summary "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn part_model_replay_allowed(part: &ShortTermMemoryPart) -> bool {
    match part.visibility.as_str() {
        "model" => true,
        "audit" | "internal" => false,
        _ => false,
    }
}

pub(crate) fn estimate_messages_tokens(messages: &[AgentMessage]) -> u32 {
    messages.iter().fold(0u32, |total, message| {
        total.saturating_add(eliteforge_agent_memory::estimate_serialized_tokens(message))
    })
}

fn hash_context_messages(messages: &[AgentMessage]) -> String {
    let mut hasher = Sha256::new();
    for message in messages {
        hasher.update(serde_json::to_vec(message).unwrap_or_default());
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())
}
