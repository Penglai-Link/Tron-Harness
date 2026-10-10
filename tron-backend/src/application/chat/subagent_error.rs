pub fn descriptor(
    error: &eliteforge_agent_core::StateError,
) -> super::history_error::ApplicationErrorDescriptor {
    let (code, message_key) = if matches!(error, eliteforge_agent_core::StateError::NotFound) {
        ("101032001", "tron.chat.subagent_not_found")
    } else {
        ("101032002", "tron.chat.subagent_read_failed")
    };
    super::history_error::ApplicationErrorDescriptor { code, message_key }
}
