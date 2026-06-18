use std::sync::{Arc, Mutex};

use crate::domain::rag::RagError;

use super::{RagConfigRepository, RagIngestScope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagDeactivateSourceCommand {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub workspace_code: String,
    pub scope: RagIngestScope,
}

pub trait RagSourceLifecycleRepository: RagConfigRepository {
    fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError>;
}

impl<T> RagSourceLifecycleRepository for Arc<Mutex<T>>
where
    T: RagSourceLifecycleRepository,
{
    fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .deactivate_source(command)
    }
}
