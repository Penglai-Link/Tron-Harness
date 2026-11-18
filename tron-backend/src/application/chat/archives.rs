use super::history_error::ApplicationErrorDescriptor;

/// Error descriptor used by archived-container operations and the topic
/// name conflict surfaced when restoring archived content.
#[derive(Clone, Copy, Debug)]
pub enum ArchiveError {
    NotFound,
    WorkspaceInactive,
    TopicNameConflict,
    Storage,
}

impl ArchiveError {
    pub fn descriptor(self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::NotFound => ("101045002", "tron.archives.not_found"),
            Self::WorkspaceInactive => ("101045003", "tron.archives.workspace_inactive"),
            Self::TopicNameConflict => ("101045005", "tron.archives.topic_name_conflict"),
            Self::Storage => ("101045004", "tron.archives.storage_failed"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}
