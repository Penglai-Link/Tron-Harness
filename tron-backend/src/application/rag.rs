mod diagnostics;
mod events;
mod facade;
mod ingest;
mod ingest_payload;
mod ingest_profile;
mod retry;
mod search;
mod search_filter;
mod search_observability;
mod search_profile;
mod service;

pub use diagnostics::{
    RagDebugProfileResult, RagDebugSourceRecord, RagDebugSourceRequest, RagDebugSourceResult,
    RagDebugTextUnitRecord, RagDiagnosticWarningRecord, RagDiagnosticsRepository,
    RagDiagnosticsService, RagSearchTraceWrite, safe_trace_payload,
};
pub use events::publish_rag_event;
pub use facade::RagService;
pub use ingest::{
    RagEmbeddingCall, RagEmbeddingRuntime, RagIngestAccessContext, RagIngestRepository,
    RagIngestRequest, RagIngestResult, RagIngestScope, RagIngestService, RagIngestWarning,
    RagMemorySourceFields, RagPersistIngestCommand, RagProjectDocSourceFields, RagTextUnitWrite,
    validate_direct_scope,
};
pub use ingest_profile::{CompiledRagIngestProfile, compile_ingest_profile};
pub use retry::{
    RagRetryIndexRepository, RagRetryIndexRequest, RagRetryIndexResult, RagRetryIndexService,
    RagRetryStageResult, RagRetryTextUnit, RagRetryVectorUpdate,
};
pub use search::{
    CandidateHit, ContextPack, ContextPackItem, RagRerankCall, RagSearchContextRequest,
    RagSearchRepository, RagSearchRequest, RagSearchResult, RagSearchRow, RagSearchRuntime,
    RagSearchScope, RagSearchService, RagSearchTraceRequest, RagTextUnitSearchQuery,
};
pub use search_filter::merge_search_filters;
pub use search_profile::{CompiledRetrievalProfile, RetrievalRoute, compile_retrieval_profile};
pub use service::{
    DefaultRagStrategyRegistry, RagConfigRepository, RagConfigService, RagCorpusView,
    RagFilterFieldError, RagFilterSchema, RagFilterSchemaField, RagFilterValidationRequest,
    RagFilterValidationView, RagFilterValueType, RagProfileListQuery, RagProfileValidationError,
    RagProfileValidationView, RagProfileView, RagStrategyReferenceView, RagStrategyRegistry,
    StrategyRegistration, StrategyRegistrationStatus,
};
