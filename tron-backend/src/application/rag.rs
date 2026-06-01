mod diagnostics;
mod document_source;
mod events;
mod facade;
mod ingest;
mod ingest_artifact;
mod ingest_payload;
mod ingest_profile;
mod retry;
mod runtime;
mod search;
mod search_filter;
mod search_observability;
mod search_profile;
mod search_ranking;
mod search_strategy;
mod service;
mod strategy_registry;

pub use diagnostics::{
    RagDebugProfileResult, RagDebugSourceRecord, RagDebugSourceRequest, RagDebugSourceResult,
    RagDebugTextUnitRecord, RagDiagnosticWarningRecord, RagDiagnosticsRepository,
    RagDiagnosticsService, RagSearchTraceWrite, safe_trace_payload,
};
pub use document_source::{RagDocumentSourceCall, RagDocumentSourceRuntime, RagResolvedDocument};
pub use events::publish_rag_event;
pub use facade::RagService;
pub use ingest::{
    RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime, RagIngestAccessContext,
    RagIngestRepository, RagIngestRequest, RagIngestResult, RagIngestScope, RagIngestService,
    RagIngestWarning, RagMemorySourceFields, RagPersistIngestCommand, RagProjectDocSourceFields,
    RagTextUnitWrite, validate_direct_scope,
};
pub use ingest_profile::{CompiledRagIngestProfile, compile_ingest_profile};
pub use retry::{
    RagRetryIndexRepository, RagRetryIndexRequest, RagRetryIndexResult, RagRetryIndexService,
    RagRetryStageResult, RagRetryTextUnit, RagRetryVectorUpdate,
};
pub use runtime::DefaultRagRuntime;
pub use search::{
    CandidateHit, ContextPack, ContextPackItem, RagRerankCall, RagRouteSearchQuery,
    RagSearchContextRequest, RagSearchRepository, RagSearchRequest, RagSearchResult, RagSearchRow,
    RagSearchRuntime, RagSearchScope, RagSearchService, RagSearchTraceRequest,
};
pub use search_filter::merge_search_filters;
pub use search_profile::{CompiledRetrievalProfile, RetrievalRoute, compile_retrieval_profile};
pub use service::{
    RagConfigRepository, RagConfigService, RagCorpusView, RagFilterFieldError, RagFilterSchema,
    RagFilterSchemaField, RagFilterValidationRequest, RagFilterValidationView, RagFilterValueType,
    RagProfileListQuery, RagProfileValidationError, RagProfileValidationView, RagProfileView,
    RagStrategyReferenceView, RagStrategyRegistry, StrategyRegistration,
    StrategyRegistrationStatus,
};
pub use strategy_registry::DefaultRagStrategyRegistry;
