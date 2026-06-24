use std::sync::Arc;

use serde_json::Value;

use crate::{
    application::{
        graph::{GraphScopeResolver, GraphStore},
        rag::{
            RagConfigRepository, RagCorpusManagementRecord, RagDeactivateSourceCommand,
            RagDebugSourceRequest, RagDebugSourceResult, RagDiagnosticsRepository,
            RagGraphExpansionQuery, RagIngestRepository, RagManagementRepository,
            RagPersistIngestCommand, RagProfileListQuery, RagRetryIndexRepository,
            RagRetryIndexRequest, RagRetryTextUnit, RagRetryVectorUpdate, RagRouteSearchQuery,
            RagSearchRepository, RagSearchRow, RagSearchTraceWrite, RagSourceLifecycleRepository,
        },
    },
    domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagStorageBackend},
};

use super::sqlite::SqliteRagRepository;

mod diagnostics;
mod ingest;
mod model;
mod search;
mod tree_rows;
mod write_adapters;
mod write_plan;

use write_adapters::RagStorageWriteAdapterRegistry;
use write_plan::RagStorageWritePlan;

pub struct GraphBackedRagRepository {
    pub(super) sqlite: SqliteRagRepository,
    pub(super) graph_store: Arc<dyn GraphStore>,
    pub(super) scope_resolver: Arc<dyn GraphScopeResolver>,
}

impl GraphBackedRagRepository {
    pub fn new(
        sqlite: SqliteRagRepository,
        graph_store: Arc<dyn GraphStore>,
        scope_resolver: Arc<dyn GraphScopeResolver>,
    ) -> Self {
        Self {
            sqlite,
            graph_store,
            scope_resolver,
        }
    }
}

impl RagConfigRepository for GraphBackedRagRepository {
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        self.sqlite.list_corpora()
    }

    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
        self.sqlite.get_corpus(corpus_id)
    }

    fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileRecord>, RagError> {
        self.sqlite.list_profiles(query)
    }

    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagError> {
        self.sqlite.get_profile(profile_id)
    }
}

impl RagManagementRepository for GraphBackedRagRepository {
    fn get_managed_corpus(
        &self,
        corpus_id: &str,
    ) -> Result<Option<RagCorpusManagementRecord>, RagError> {
        self.sqlite.get_managed_corpus(corpus_id)
    }

    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError> {
        self.sqlite.save_managed_corpus(record)
    }

    fn save_profile(&self, record: &RagProfileRecord) -> Result<(), RagError> {
        self.sqlite.save_profile(record)
    }
}

impl RagIngestRepository for GraphBackedRagRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return self.sqlite.persist_ingest(command);
        }
        let plan = RagStorageWritePlan::compile(&corpus)?;
        RagStorageWriteAdapterRegistry::new(self).execute(&plan, &corpus, &command)
    }
}

impl RagSourceLifecycleRepository for GraphBackedRagRepository {
    fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return self.sqlite.deactivate_source(command);
        }
        ensure_graph_runtime_supported(&corpus)?;
        self.deactivate_graph_single_node(&corpus, command)
    }
}

impl RagSearchRepository for GraphBackedRagRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&query.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return self.sqlite.search_route(query);
        }
        if is_tree_retriever(&query.retriever) {
            return match corpus
                .storage_binding
                .target_for_segment("tree")
                .map(|target| target.backend)
            {
                Some(RagStorageBackend::Sqlite) => self.sqlite.search_route(query),
                Some(RagStorageBackend::Graph) => {
                    ensure_graph_runtime_supported(&corpus)?;
                    self.search_graph_route(&corpus, &query)
                }
                None => Err(RagError::SearchRouteUnavailable),
            };
        }
        if is_sqlite_retriever(&query.retriever) {
            if !has_sqlite_runtime_tables(&corpus) {
                return Err(RagError::SearchRouteUnavailable);
            }
            return self.sqlite.search_route(query);
        }
        if !is_graph_retriever(&query.retriever) {
            return Err(RagError::SearchRouteUnavailable);
        }
        ensure_graph_runtime_supported(&corpus)?;
        let rows = self.search_graph_route(&corpus, &query)?;
        if has_sqlite_runtime_tables(&corpus) {
            self.sqlite
                .hydrate_graph_rows_from_text_units(&corpus, rows)
        } else {
            Ok(rows)
        }
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        RagSearchRepository::persist_search_trace(&self.sqlite, trace)
    }

    fn expand_related_graph(
        &self,
        query: RagGraphExpansionQuery,
    ) -> Result<Option<Value>, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&query.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return Ok(None);
        }
        ensure_graph_runtime_supported(&corpus)?;
        self.expand_graph_hit(&corpus, &query)
    }
}

impl RagDiagnosticsRepository for GraphBackedRagRepository {
    fn debug_source(
        &self,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return self.sqlite.debug_source(request);
        }
        ensure_graph_runtime_supported(&corpus)?;
        self.debug_graph_source(&corpus, request)
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        RagDiagnosticsRepository::persist_search_trace(&self.sqlite, trace)
    }
}

impl RagRetryIndexRepository for GraphBackedRagRepository {
    fn retry_text_units(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<Vec<RagRetryTextUnit>, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return self.sqlite.retry_text_units(request);
        }
        ensure_graph_runtime_supported(&corpus)?;
        if request.stages.iter().any(|stage| stage == "vectorIndex") {
            return Err(RagError::RetryStageUnsupported);
        }
        let Some(node) = self.find_graph_main_node(&corpus, request)? else {
            return Ok(Vec::new());
        };
        let text = model::graph_text(&node);
        Ok(vec![RagRetryTextUnit {
            text_unit_code: node.node_code.clone(),
            index_text: text.clone(),
            text,
        }])
    }

    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if !corpus.storage_binding.is_graph_backend() {
            return self.sqlite.rebuild_fulltext_index(request);
        }
        ensure_graph_runtime_supported(&corpus)?;
        Ok(usize::from(
            self.find_graph_main_node(&corpus, request)?.is_some(),
        ))
    }

    fn update_vector_index(&self, update: RagRetryVectorUpdate) -> Result<(), RagError> {
        self.sqlite.update_vector_index(update)
    }

    fn mark_vector_retry_failed(&self, request: &RagRetryIndexRequest) -> Result<(), RagError> {
        self.sqlite.mark_vector_retry_failed(request)
    }

    fn persist_retry_warning(
        &self,
        request: &RagRetryIndexRequest,
        stage_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), RagError> {
        self.sqlite
            .persist_retry_warning(request, stage_id, code, message)
    }
}

fn ensure_graph_runtime_supported(corpus: &RagCorpusRecord) -> Result<(), RagError> {
    if model::has_graph_targets(corpus) {
        Ok(())
    } else {
        Err(RagError::StorageBindingInvalid)
    }
}

fn has_sqlite_runtime_tables(corpus: &RagCorpusRecord) -> bool {
    ["textUnit", "fulltext", "vector"]
        .into_iter()
        .any(|segment| {
            corpus
                .storage_binding
                .sqlite_table_for_segment(segment)
                .is_some()
        })
}

fn is_graph_retriever(retriever: &str) -> bool {
    matches!(
        retriever,
        "FULLTEXT_GRAPH_SINGLE_NODE_V1"
            | "VECTOR_GRAPH_SINGLE_NODE_V1"
            | "TREE_FULLTEXT_V1"
            | "TREE_VECTOR_V1"
    )
}

fn is_tree_retriever(retriever: &str) -> bool {
    matches!(retriever, "TREE_FULLTEXT_V1" | "TREE_VECTOR_V1")
}

fn is_sqlite_retriever(retriever: &str) -> bool {
    matches!(
        retriever,
        "FULLTEXT"
            | "FULLTEXT_SQLITE_LIKE_V1"
            | "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"
            | "VECTOR"
            | "VECTOR_SQLITE_COSINE_V1"
            | "VECTOR_SQLITE_VEC0_COSINE_V1"
            | "STRUCTURED_FILTER_SQLITE_V1"
    )
}
