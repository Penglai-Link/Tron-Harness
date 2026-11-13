//! Map persisted submission snapshots to a configured data contributor without
//! changing the authoritative conversation or its tool protocol identities.
use super::messages;
use crate::application::agent::page_context::is_snapshot;
use crate::domain::agent::AgentMessage;
use eliteforge_agent_core as fw;

pub(super) fn project(
    history: &mut fw::ConversationSnapshot,
) -> Result<Vec<AgentMessage>, fw::AgentError> {
    let mut current = Vec::new();
    for message in &mut history.messages {
        if message.role != fw::Role::User {
            continue;
        }
        let mut source = messages::decode(message).map_err(fw::AgentError::InvalidConfiguration)?;
        // A new input without a snapshot clears the current location. Never
        // fall back to a former message, including after a steer or restart.
        current = source
            .parts
            .iter()
            .filter(|part| is_snapshot(part))
            .cloned()
            .collect();
        if !current.is_empty() {
            source.parts.retain(|part| !is_snapshot(part));
            message.content = messages::encode(&source)
                .map_err(fw::AgentError::InvalidConfiguration)?
                .content;
        }
    }
    Ok(if current.is_empty() {
        Vec::new()
    } else {
        vec![AgentMessage::new("user", current)]
    })
}
