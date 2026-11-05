pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
pub fn descriptor(error: &eliteforge_agent_core::StateError) -> ApplicationErrorDescriptor {
    use eliteforge_agent_core::StateError;
    let (code, message_key) = match error {
        StateError::Invalid(_) => ("101030001", "tron.chat.history_query_invalid"),
        StateError::Conflict => ("101030002", "tron.chat.history_version_changed"),
        StateError::NotFound => ("101030003", "tron.chat.history_not_found"),
        _ => ("101030004", "tron.chat.history_read_failed"),
    };
    ApplicationErrorDescriptor { code, message_key }
}

pub fn deletion_descriptor(error: &eliteforge_agent_core::StateError) -> ApplicationErrorDescriptor {
    use eliteforge_agent_core::StateError;
    let (code, message_key) = match error {
        StateError::UnsupportedVersion => ("101030008", "tron.chat.history_delete_unsupported"),
        StateError::Busy => ("101030005", "tron.chat.history_delete_busy"),
        StateError::Invalid(_) => ("101030006", "tron.chat.history_delete_invalid"),
        StateError::Conflict => ("101030002", "tron.chat.history_version_changed"),
        StateError::NotFound => ("101030003", "tron.chat.history_not_found"),
        _ => ("101030007", "tron.chat.history_delete_failed"),
    };
    ApplicationErrorDescriptor { code, message_key }
}
