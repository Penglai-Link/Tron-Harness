use super::history_error::ApplicationErrorDescriptor;

#[derive(Clone, Copy, Debug)]
pub enum ArchivedContainerKind {
    Topic,
    Project,
}

pub struct ArchivedContainer {
    pub code: String,
    pub workspace_code: String,
    pub name: String,
    pub path: Option<String>,
    pub chat_count: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum ArchiveError {
    InvalidQuery,
    NotFound,
    WorkspaceInactive,
    TopicNameConflict,
    Storage,
}

impl ArchiveError {
    pub fn descriptor(self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::InvalidQuery => ("101045001", "tron.archives.invalid_query"),
            Self::NotFound => ("101045002", "tron.archives.not_found"),
            Self::WorkspaceInactive => ("101045003", "tron.archives.workspace_inactive"),
            Self::TopicNameConflict => ("101045005", "tron.archives.topic_name_conflict"),
            Self::Storage => ("101045004", "tron.archives.storage_failed"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}
