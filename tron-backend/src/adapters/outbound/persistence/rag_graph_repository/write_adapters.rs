use crate::{
    application::rag::{RagIngestRepository, RagPersistIngestCommand},
    domain::rag::{RagCorpusRecord, RagError, RagStorageBackend},
};

use super::{
    GraphBackedRagRepository, ensure_graph_runtime_supported,
    write_plan::{RagStorageWriteOperation, RagStorageWritePlan},
};

pub(super) struct RagStorageWriteAdapterRegistry<'a> {
    repository: &'a GraphBackedRagRepository,
    adapters: Vec<RagStorageWriteAdapter>,
}

impl<'a> RagStorageWriteAdapterRegistry<'a> {
    pub(super) fn new(repository: &'a GraphBackedRagRepository) -> Self {
        Self {
            repository,
            adapters: vec![
                RagStorageWriteAdapter::Sqlite,
                RagStorageWriteAdapter::Graph,
            ],
        }
    }

    pub(super) fn execute(
        &self,
        plan: &RagStorageWritePlan,
        corpus: &RagCorpusRecord,
        command: &RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        for adapter in &self.adapters {
            let operations = plan
                .operations
                .iter()
                .filter(|operation| adapter.supports(operation))
                .collect::<Vec<_>>();
            if !operations.is_empty() {
                adapter.execute(self.repository, &operations, corpus, command)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RagStorageWriteAdapter {
    Sqlite,
    Graph,
}

impl RagStorageWriteAdapter {
    fn supports(&self, operation: &RagStorageWriteOperation) -> bool {
        !operation.segments.is_empty()
            && matches!(
                (self, operation.backend),
                (Self::Sqlite, RagStorageBackend::Sqlite) | (Self::Graph, RagStorageBackend::Graph)
            )
    }

    fn execute(
        &self,
        repository: &GraphBackedRagRepository,
        operations: &[&RagStorageWriteOperation],
        corpus: &RagCorpusRecord,
        command: &RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        match self {
            Self::Sqlite => execute_sqlite_write(repository, operations, corpus, command),
            Self::Graph => execute_graph_write(repository, operations, corpus, command),
        }
    }
}

fn execute_sqlite_write(
    repository: &GraphBackedRagRepository,
    operations: &[&RagStorageWriteOperation],
    corpus: &RagCorpusRecord,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    if operations
        .iter()
        .any(|operation| operation.segments.contains("tree"))
    {
        if corpus
            .storage_binding
            .sqlite_table_for_segment("textUnit")
            .is_none()
        {
            return Err(RagError::StorageTopologyUnsupported);
        }
        return repository.sqlite.persist_ingest(command.clone());
    }
    if operations
        .iter()
        .any(|operation| operation.segments.iter().any(is_retrieval_segment))
    {
        return repository.sqlite.persist_ingest(command.clone());
    }
    if operations
        .iter()
        .any(|operation| operation.segments.contains("source"))
    {
        return repository.sqlite.persist_source_only(corpus, command);
    }
    Ok(())
}

fn execute_graph_write(
    repository: &GraphBackedRagRepository,
    operations: &[&RagStorageWriteOperation],
    corpus: &RagCorpusRecord,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    ensure_graph_runtime_supported(corpus)?;
    if operations.len() > 1
        || operations
            .iter()
            .any(|operation| operation.segments.contains("tree"))
    {
        repository.persist_graph_segmented(corpus, command.clone())
    } else {
        repository.persist_graph_single_node(corpus, command.clone())
    }
}

fn is_retrieval_segment(segment: &String) -> bool {
    matches!(segment.as_str(), "textUnit" | "fulltext" | "vector")
}
