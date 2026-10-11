use crate::domain::dude::DudeError;
pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
pub fn error_descriptor(error: &DudeError) -> ApplicationErrorDescriptor {
    let (code, message_key) = match error {
        DudeError::NotFound => ("101018001", "tron.dude.not_found"),
        DudeError::Invalid => ("101018002", "tron.dude.request_invalid"),
        DudeError::Conflict => ("101018003", "tron.dude.state_changed"),
        DudeError::Storage => ("101018004", "tron.dude.storage_failed"),
        DudeError::Runtime(_) => ("101018005", "tron.dude.runtime_failed"),
    };
    ApplicationErrorDescriptor { code, message_key }
}
