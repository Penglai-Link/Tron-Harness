mod diagnostics;
mod document_source;
mod events;
mod facade;
mod ingest;
mod ingest_artifact;
mod ingest_payload;
mod ingest_profile;
mod lifecycle;
mod management;
mod mineru;
mod pipeline;
mod retry;
mod runtime;
mod search;
mod search_filter;
mod search_observability;
mod search_profile;
mod search_ranking;
mod search_strategy;
mod search_validation;
mod service;
mod strategy_registry;

pub use diagnostics::{
    RagDebugProfileResult, RagDebugSourceRecord, RagDebugSourceRequest, RagDebugSourceResult,
    RagDebugTextUnitRecord, RagDiagnosticWarningRecord, RagDiagnosticsRepository,
    RagDiagnosticsService, RagSearchTraceWrite, safe_trace_payload,
};
pub use document_source::{
    RagDocumentParseCall, RagDocumentSourceCall, RagDocumentSourceRuntime, RagResolvedDocument,
};
pub use events::publish_rag_event;
pub use facade::RagService;
pub use ingest::{
    RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime, RagIngestAccessContext,
    RagIngestRepository, RagIngestRequest, RagIngestResult, RagIngestScope, RagIngestService,
    RagIngestSource, RagIngestWarning, RagPersistIngestCommand, RagTextUnitWrite, RagTreeNodeWrite,
    RagTreeSummaryCall, RagTreeSummaryResult, RagTreeSummaryRuntime, validate_direct_scope,
};
pub use ingest_profile::{
    CompiledRagIngestProfile, CompiledRagIngestStage, compile_ingest_profile,
};
pub use lifecycle::{RagDeactivateSourceCommand, RagSourceLifecycleRepository};
pub use management::*;
pub use pipeline::{
    CompiledRagPipeline, CompiledRagPipelineNode, RagPipelineCatalogView, RagPipelineEdgeInput,
    RagPipelineInput, RagPipelineNodeInput, RagPipelineValidationIssue, RagStageExecutionResult,
    execution_stage_for_stage_type, pipeline_catalog, stage_result_from_definition,
    stage_type_for_execution_stage, validate_pipeline,
};
pub(crate) use pipeline::{RagArtifactStore, RagPipelineExecutor, RagStageOutcome};
pub use retry::{
    RagRetryIndexRepository, RagRetryIndexRequest, RagRetryIndexResult, RagRetryIndexService,
    RagRetryStageResult, RagRetryTextUnit, RagRetryVectorUpdate,
};
pub use runtime::DefaultRagRuntime;
pub use search::{
    CandidateHit, ContextPack, ContextPackItem, RagGraphExpansionQuery, RagRerankCall,
    RagRouteSearchQuery, RagSearchContextRequest, RagSearchGraphExpansionRequest,
    RagSearchRepository, RagSearchRequest, RagSearchResult, RagSearchRow, RagSearchRuntime,
    RagSearchScope, RagSearchService, RagSearchTraceRequest,
};
pub use search_filter::{
    merge_search_filters, merge_search_request_filters, validate_request_filters_for_corpora,
};
pub use search_profile::{
    CompiledRetrievalProfile, DedupPolicy, EmbeddingMmrPolicy, FusionPolicy, RetrievalRoute,
    TopicFallback, TopicPolicy, compile_retrieval_profile,
};
pub use service::{
    RagConfigRepository, RagConfigService, RagCorpusView, RagFilterFieldError, RagFilterSchema,
    RagFilterSchemaField, RagFilterValidationRequest, RagFilterValidationView, RagFilterValueType,
    RagProfileListQuery, RagProfileValidationError, RagProfileValidationView, RagProfileView,
    RagStrategyReferenceView, RagStrategyRegistry, StrategyRegistration,
    StrategyRegistrationStatus,
};
pub use strategy_registry::DefaultRagStrategyRegistry;
