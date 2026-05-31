mod ingest;
mod ingest_payload;
mod ingest_profile;
mod search;
mod search_filter;
mod search_profile;
mod service;

pub use ingest::{
    RagEmbeddingCall, RagEmbeddingRuntime, RagIngestAccessContext, RagIngestRepository,
    RagIngestRequest, RagIngestResult, RagIngestScope, RagIngestService, RagIngestWarning,
    RagMemorySourceFields, RagPersistIngestCommand, RagProjectDocSourceFields, RagTextUnitWrite,
    validate_direct_scope,
};
pub use ingest_profile::{CompiledRagIngestProfile, compile_ingest_profile};
pub use search::{
    CandidateHit, ContextPack, ContextPackItem, RagRerankCall, RagSearchContextRequest,
    RagSearchRepository, RagSearchRequest, RagSearchResult, RagSearchRow, RagSearchRuntime,
    RagSearchScope, RagSearchService, RagTextUnitSearchQuery,
};
pub use search_filter::merge_search_filters;
pub use search_profile::{CompiledRetrievalProfile, RetrievalRoute, compile_retrieval_profile};
pub use service::{
    DefaultRagStrategyRegistry, RagConfigRepository, RagFilterFieldError, RagFilterSchema,
    RagFilterSchemaField, RagFilterValidationRequest, RagFilterValidationView, RagFilterValueType,
    RagProfileListQuery, RagProfileValidationError, RagProfileValidationView, RagProfileView,
    RagService, RagStrategyReferenceView, RagStrategyRegistry, StrategyRegistration,
    StrategyRegistrationStatus,
};
