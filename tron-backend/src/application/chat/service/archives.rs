use super::ChatService;
use crate::application::chat::{ArchiveError, ArchivedContainer, ArchivedContainerKind};

impl ChatService {
    pub fn archived_containers(
        &self,
        kind: ArchivedContainerKind,
        workspace_code: Option<&str>,
        current: u32,
        size: u32,
    ) -> Result<(Vec<ArchivedContainer>, u64), ArchiveError> {
        if current == 0 || size == 0 || size > 100 {
            return Err(ArchiveError::InvalidQuery);
        }
        self.repository().archived_containers(
            kind,
            workspace_code.filter(|code| !code.is_empty()),
            current.saturating_sub(1).saturating_mul(size),
            size,
        )
    }

    pub fn restore_archived_container(
        &self,
        kind: ArchivedContainerKind,
        code: &str,
    ) -> Result<(), ArchiveError> {
        // Serialize with container archive/delete and chat placement changes.
        let _admission = self
            .active_runs
            .lock_admission()
            .map_err(|_| ArchiveError::Storage)?;
        self.repository().restore_archived_container(kind, code)
    }
}
