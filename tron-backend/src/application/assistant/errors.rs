use crate::domain::assistant::AssistantError;
pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
pub fn error_descriptor(error: &AssistantError) -> ApplicationErrorDescriptor {
    let (code, message_key) = match error {
        AssistantError::NotFound => ("101018001", "tron.assistant.not_found"),
        AssistantError::Invalid => ("101018002", "tron.assistant.request_invalid"),
        AssistantError::Conflict | AssistantError::ExecutionBusy => {
            ("101018003", "tron.assistant.state_changed")
        }
        AssistantError::Storage => ("101018004", "tron.assistant.storage_failed"),
        AssistantError::Runtime(_) => ("101018005", "tron.assistant.runtime_failed"),
    };
    ApplicationErrorDescriptor { code, message_key }
}
