use super::history_error::ApplicationErrorDescriptor;
pub fn descriptor(error: &eliteforge_agent_core::AgentError) -> ApplicationErrorDescriptor {
    use eliteforge_agent_core::{AgentError, StateError};
    let (code, message_key) = match error {
        AgentError::State(StateError::Conflict) => ("101031002", "tron.chat.a2ui_conflict"),
        AgentError::State(StateError::NotFound) => ("101031003", "tron.chat.a2ui_not_found"),
        AgentError::InvalidConfiguration(_) => ("101031001", "tron.chat.a2ui_invalid"),
        _ => ("101031004", "tron.chat.a2ui_unavailable"),
    };
    ApplicationErrorDescriptor { code, message_key }
}
