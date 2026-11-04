use crate::domain::{
    agent::{AgentMessage, AgentMessagePart},
    short_term_memory::{ShortTermMemoryGoalView, ShortTermMemoryTodo, TodoStatus},
};
pub(super) fn working_state_message(
    active_goal: Option<&ShortTermMemoryGoalView>,
    todos: &[ShortTermMemoryTodo],
    todo_enabled: bool,
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
        for todo in active_todos {
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

pub(crate) fn estimate_messages_tokens(messages: &[AgentMessage]) -> u32 {
    messages.iter().fold(0u32, |total, message| {
        total.saturating_add(eliteforge_agent_memory::estimate_serialized_tokens(message))
    })
}
