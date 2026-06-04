use std::sync::Arc;

use serde_json::Value;

use crate::{
    application::{
        graph::{GraphScopeResolver, GraphStore},
        rag::{
            RagConfigRepository, RagDebugSourceRequest, RagDebugSourceResult,
            RagDiagnosticsRepository, RagGraphExpansionQuery, RagIngestRepository,
            RagPersistIngestCommand, RagProfileListQuery, RagRetryIndexRepository,
            RagRetryIndexRequest, RagRetryTextUnit, RagRetryVectorUpdate, RagRouteSearchQuery,
            RagSearchRepository, RagSearchRow, RagSearchTraceWrite,
        },
    },
    domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagStorageLayout},
};

use super::sqlite::SqliteRagRepository;

mod diagnostics;
mod ingest;
mod model;
mod search;

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

impl RagIngestRepository for GraphBackedRagRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if corpus.storage_binding.layout != RagStorageLayout::GraphSingleNode {
            return self.sqlite.persist_ingest(command);
        }
        self.persist_graph_single_node(&corpus, command)
    }
}

impl RagSearchRepository for GraphBackedRagRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&query.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if corpus.storage_binding.layout != RagStorageLayout::GraphSingleNode {
            return self.sqlite.search_route(query);
        }
        self.search_graph_route(&corpus, &query)
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
        if corpus.storage_binding.layout != RagStorageLayout::GraphSingleNode {
            return Ok(None);
        }
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
        if corpus.storage_binding.layout != RagStorageLayout::GraphSingleNode {
            return self.sqlite.debug_source(request);
        }
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
        if corpus.storage_binding.layout != RagStorageLayout::GraphSingleNode {
            return self.sqlite.retry_text_units(request);
        }
        if request.stages.iter().any(|stage| stage == "vectorIndex") {
            return Err(RagError::RetryStageUnsupported);
        }
        let Some(node) = self.find_graph_skill_node(&corpus, request)? else {
            return Ok(Vec::new());
        };
        Ok(vec![RagRetryTextUnit {
            text_unit_code: node.node_code.clone(),
            text: model::graph_text(&node),
        }])
    }

    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError> {
        let corpus = self
            .sqlite
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if corpus.storage_binding.layout != RagStorageLayout::GraphSingleNode {
            return self.sqlite.rebuild_fulltext_index(request);
        }
        Ok(usize::from(
            self.find_graph_skill_node(&corpus, request)?.is_some(),
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
