use std::collections::BTreeMap;
use std::fs;

use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::rag::{
        DefaultRagStrategyRegistry, RagConfigRepository, RagConfigService,
        RagFilterValidationRequest, RagFilterValueType, RagProfileListQuery, RagStrategyRegistry,
        StrategyRegistration, StrategyRegistrationStatus,
    },
    domain::rag::{
        RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus, RagStorageBackend,
        RagStorageBinding, RagStorageSegmentTarget,
    },
};

#[test]
fn rag_service_resolves_enabled_corpus_storage_binding_from_repository_port() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let corpus = service
        .get_corpus("memory-card")
        .expect("memory-card corpus should exist");

    assert_eq!(corpus.corpus_id, "memory-card");
    assert_eq!(corpus.status, RagStatus::Enabled);
    assert_eq!(
        corpus.storage_binding.sqlite_table_for_segment("textUnit"),
        Some("rag_memory_text_units")
    );
    assert_eq!(
        corpus.storage_binding.sqlite_table_for_segment("source"),
        Some("rag_memory_text_units")
    );
    assert_eq!(
        corpus.storage_binding.diagnostics_table(),
        Some("rag_diagnostics")
    );
    assert_eq!(
        corpus.field_mapping.get("sourceCode").map(String::as_str),
        Some("source_code")
    );
    assert!(corpus.filter_schema.fields.contains_key("scopeType"));
    assert_eq!(
        corpus
            .filter_schema
            .fields
            .get("status")
            .and_then(|field| field.column.as_deref()),
        Some("source_status")
    );
}

#[test]
fn memory_corpus_seed_maps_governance_logical_fields_to_physical_columns() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let corpus = service
        .get_corpus("memory-card")
        .expect("memory-card corpus should exist");

    for (field, column, field_type) in [
        ("memoryType", "memory_type", RagFilterValueType::String),
        ("memoryStatus", "memory_status", RagFilterValueType::String),
        ("reviewStatus", "review_status", RagFilterValueType::String),
        ("version", "version", RagFilterValueType::String),
        ("supersededBy", "superseded_by", RagFilterValueType::String),
        (
            "conflictGroup",
            "conflict_group",
            RagFilterValueType::String,
        ),
        (
            "duplicateGroup",
            "duplicate_group",
            RagFilterValueType::String,
        ),
        ("injectable", "injectable", RagFilterValueType::Boolean),
        (
            "importanceScore",
            "importance_score",
            RagFilterValueType::Integer,
        ),
        (
            "confidenceScore",
            "confidence_score",
            RagFilterValueType::Integer,
        ),
        ("qualityScore", "quality_score", RagFilterValueType::Integer),
        ("pinLevel", "pin_level", RagFilterValueType::Integer),
        ("expiresAt", "expires_at", RagFilterValueType::Integer),
        (
            "derivedSourceType",
            "derived_source_type",
            RagFilterValueType::String,
        ),
        (
            "derivedSourceCode",
            "derived_source_code",
            RagFilterValueType::String,
        ),
    ] {
        assert_eq!(
            corpus.field_mapping.get(field).map(String::as_str),
            Some(column),
            "{field} should map to {column}",
        );
        let schema_field = corpus
            .filter_schema
            .fields
            .get(field)
            .unwrap_or_else(|| panic!("{field} should be declared in filterSchema"));
        assert_eq!(schema_field.column.as_deref(), Some(column));
        assert_eq!(schema_field.field_type, field_type);
    }

    assert!(
        corpus
            .filter_schema
            .fields
            .get("pinLevel")
            .expect("pinLevel should be declared")
            .operators
            .contains("gte"),
        "numeric Memory fields should support range filtering",
    );
}

#[test]
fn project_doc_corpus_seed_maps_repo_module_logical_fields_to_physical_columns() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let corpus = service
        .get_corpus("project-doc")
        .expect("project-doc corpus should exist");

    for (field, column) in [
        ("documentCode", "document_code"),
        ("documentType", "document_type"),
        ("path", "path"),
        ("repoCode", "repo_code"),
        ("moduleCode", "module_code"),
        ("moduleName", "module_name"),
    ] {
        assert_eq!(
            corpus.field_mapping.get(field).map(String::as_str),
            Some(column),
            "{field} should map to {column}",
        );
        let schema_field = corpus
            .filter_schema
            .fields
            .get(field)
            .unwrap_or_else(|| panic!("{field} should be declared in filterSchema"));
        assert_eq!(schema_field.column.as_deref(), Some(column));
        assert_eq!(schema_field.field_type, RagFilterValueType::String);
    }
}

#[test]
fn ingest_profiles_declare_contract_payload_strategies_for_memory_and_project_doc() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let memory = service
        .get_profile("memory-card-ingest-v1")
        .expect("memory ingest profile should be readable");
    assert_eq!(
        memory.config["stages"]["preparePayload"]["strategyId"],
        json!("RAG_TEXT_CONTRACT_PAYLOAD_V1")
    );
    assert_eq!(
        memory.config["sourceInputPolicy"]["supportedSourceTypes"],
        json!(["MEMORY_CARD"])
    );
    assert_eq!(
        memory.degrade_policy, "SKIP_VECTOR_WITH_WARNING",
        "memory-card ingest should keep fulltext persistence available when vector embedding is unavailable",
    );

    let project_doc = service
        .get_profile("project-doc-ingest-v1")
        .expect("project-doc ingest profile should be readable");
    assert_eq!(
        project_doc.config["stages"]["preparePayload"]["strategyId"],
        json!("RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1")
    );
    assert_eq!(
        project_doc.config["stages"]["parseDocument"]["config"]["format"],
        json!("MARKDOWN")
    );
}

#[test]
fn rag_service_lists_and_reads_profile_configuration_without_sqlite_details() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let profiles = service
        .list_profiles(RagProfileListQuery {
            profile_type: Some(RagProfileType::Retrieval),
            status: Some(RagStatus::Enabled),
            corpus_id: Some("memory-card".to_string()),
        })
        .expect("profile list should load through the repository port");

    let profile_ids = profiles
        .iter()
        .map(|profile| profile.profile_id.as_str())
        .collect::<Vec<_>>();
    assert!(profile_ids.contains(&"memory-related-v1"));
    assert!(profile_ids.contains(&"memory-query-v1"));
    assert!(profile_ids.contains(&"memory-project-doc-fusion-v1"));
    assert!(
        profiles
            .iter()
            .all(|profile| profile.required_source_version.as_deref() == Some("v1"))
    );

    let memory_query = service
        .get_profile("memory-query-v1")
        .expect("memory query profile should be readable");
    let route_retrievers = memory_query.config["routes"]
        .as_array()
        .expect("memory query profile should declare routes")
        .iter()
        .map(|route| route["retriever"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(
        route_retrievers,
        vec![
            "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
            "VECTOR_SQLITE_VEC0_COSINE_V1"
        ]
    );
    assert!(
        !route_retrievers.contains(&"STRUCTURED_FILTER_SQLITE_V1"),
        "memory-query-v1 must stay query-only and avoid broad structured filter retrieval"
    );
    assert_eq!(
        memory_query.config["contextPolicy"]["supportedStrategies"],
        json!(["RESULT_LIST"])
    );

    let profile = service
        .get_profile("memory-related-v1")
        .expect("seeded retrieval profile should be readable");
    assert_eq!(profile.primary_corpus_id.as_deref(), Some("memory-card"));
    assert_eq!(profile.corpus_ids, vec!["memory-card".to_string()]);
    assert_eq!(profile.degrade_policy, "SKIP_VECTOR_ON_UNAVAILABLE");
    let retrievers = profile.config["routes"]
        .as_array()
        .expect("memory retrieval routes should be an array")
        .iter()
        .map(|route| route["retriever"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        retrievers,
        vec![
            json!("STRUCTURED_FILTER_SQLITE_V1"),
            json!("FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
            json!("VECTOR_SQLITE_VEC0_COSINE_V1"),
        ]
    );

    let fusion_profile = service
        .get_profile("memory-project-doc-fusion-v1")
        .expect("multi-corpus retrieval profile should be readable");
    assert_eq!(
        fusion_profile.primary_corpus_id.as_deref(),
        Some("memory-card")
    );
    assert_eq!(
        fusion_profile.corpus_ids,
        vec!["memory-card".to_string(), "project-doc".to_string()]
    );
    assert_eq!(fusion_profile.config["routes"].as_array().unwrap().len(), 4);
    assert_eq!(fusion_profile.config["fusionPolicy"]["type"], json!("RRF"));
    assert_eq!(
        fusion_profile.config["dedupPolicy"]["type"],
        json!("CANONICAL_TEXT_UNIT")
    );
    assert_eq!(
        fusion_profile.config["topicPolicy"]["type"],
        json!("EMBEDDING_MMR_V1")
    );
    assert_eq!(
        fusion_profile.config["rerankPolicy"]["enabled"],
        json!(false)
    );

    let three_way_fusion_profile = service
        .get_profile("skill-memory-project-fusion-v1")
        .expect("skill memory project fusion profile should be readable");
    assert_eq!(
        three_way_fusion_profile.primary_corpus_id.as_deref(),
        Some("skill-graph")
    );
    assert_eq!(
        three_way_fusion_profile.corpus_ids,
        vec![
            "skill-graph".to_string(),
            "memory-card".to_string(),
            "project-doc".to_string()
        ]
    );
    let routes = three_way_fusion_profile.config["routes"]
        .as_array()
        .expect("three-way fusion profile should expose routes");
    assert_eq!(routes.len(), 6);
    assert!(routes.iter().any(|route| {
        route["corpusId"] == json!("skill-graph")
            && route["retriever"] == json!("FULLTEXT_GRAPH_SINGLE_NODE_V1")
    }));
    assert!(routes.iter().any(|route| {
        route["corpusId"] == json!("skill-graph")
            && route["retriever"] == json!("VECTOR_GRAPH_SINGLE_NODE_V1")
    }));
    assert!(routes.iter().any(|route| {
        route["corpusId"] == json!("memory-card")
            && route["retriever"] == json!("FULLTEXT_SQLITE_FTS5_SIMPLE_V1")
    }));
    assert!(routes.iter().any(|route| {
        route["corpusId"] == json!("project-doc")
            && route["retriever"] == json!("VECTOR_SQLITE_VEC0_COSINE_V1")
    }));
    assert_eq!(
        three_way_fusion_profile.config["fusionPolicy"]["type"],
        json!("RRF")
    );
    assert_eq!(
        three_way_fusion_profile.config["graphExpansionPolicy"]["enabled"],
        json!(true)
    );
}

#[test]
fn storage_backend_domain_accepts_only_sqlite_or_graph_codes() {
    assert_eq!(
        RagStorageBackend::try_from("SQLITE").unwrap(),
        RagStorageBackend::Sqlite
    );
    assert_eq!(
        RagStorageBackend::try_from("GRAPH").unwrap(),
        RagStorageBackend::Graph
    );
    assert_eq!(
        RagStorageBackend::try_from("graph").unwrap_err(),
        RagError::StorageBindingInvalid
    );
    assert_eq!(
        RagStorageBackend::try_from("ELASTIC").unwrap_err(),
        RagError::StorageBindingInvalid
    );
}

#[test]
fn skill_retrieval_profile_uses_same_extension_retrievers_as_memory() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let profile = service
        .get_profile("skill-related-v1")
        .expect("skill retrieval profile should be readable");

    assert_eq!(profile.primary_corpus_id.as_deref(), Some("skill"));
    assert_eq!(
        profile.config["routes"][0]["retriever"],
        json!("FULLTEXT_SQLITE_FTS5_SIMPLE_V1")
    );
    assert_eq!(
        profile.config["routes"][1]["retriever"],
        json!("VECTOR_SQLITE_VEC0_COSINE_V1")
    );
}

#[test]
fn retrieval_profile_validation_rejects_missing_or_latest_source_version() {
    let service = RagConfigService::new(
        FakeRagRepository::with_profiles(vec![
            profile_record(
                "missing-version",
                None,
                json!({ "retrievers": ["FULLTEXT"] }),
            ),
            profile_record(
                "latest-version",
                Some("latest"),
                json!({ "retrievers": ["FULLTEXT"] }),
            ),
            profile_record(
                "fixed-version",
                Some("v1"),
                json!({ "retrievers": ["FULLTEXT"] }),
            ),
        ]),
        DefaultRagStrategyRegistry::default(),
    );

    let missing = service.validate_profile("missing-version").unwrap();
    assert!(!missing.valid);
    assert!(missing.errors.iter().any(|error| {
        error.code == "RAG_PROFILE_SOURCE_VERSION_REQUIRED"
            && error.message.contains("requiredSourceVersion")
    }));

    let latest = service.validate_profile("latest-version").unwrap();
    assert!(!latest.valid);
    assert!(latest.errors.iter().any(|error| {
        error.code == "RAG_PROFILE_SOURCE_VERSION_REQUIRED" && error.message.contains("latest")
    }));

    let fixed = service.validate_profile("fixed-version").unwrap();
    assert!(fixed.valid, "{fixed:?}");
}

#[test]
fn strategy_registry_validation_reports_enabled_reserved_and_unsupported_references() {
    let registry = FakeStrategyRegistry::new([
        ("FULLTEXT", StrategyRegistrationStatus::Enabled),
        ("GRAPH", StrategyRegistrationStatus::Reserved),
        ("UNKNOWN_RETRIEVER", StrategyRegistrationStatus::Unsupported),
    ]);
    let service = RagConfigService::new(
        FakeRagRepository::with_profiles(vec![profile_record(
            "mixed-strategies",
            Some("v1"),
            json!({
                "stages": ["INGEST_CHUNK"],
                "retrievers": ["FULLTEXT", "GRAPH", "UNKNOWN_RETRIEVER"],
                "policies": ["SKIP_VECTOR_ON_UNAVAILABLE"]
            }),
        )]),
        registry,
    );

    let validation = service.validate_profile("mixed-strategies").unwrap();

    assert!(!validation.valid);
    assert!(validation.strategy_refs.iter().any(|reference| {
        reference.kind == "retriever"
            && reference.strategy_id == "FULLTEXT"
            && reference.status == StrategyRegistrationStatus::Enabled
    }));
    assert!(validation.errors.iter().any(|error| {
        error.code == "RAG_STRATEGY_RESERVED" && error.strategy_id.as_deref() == Some("GRAPH")
    }));
    assert!(validation.errors.iter().any(|error| {
        error.code == "RAG_STRATEGY_UNREGISTERED"
            && error.strategy_id.as_deref() == Some("UNKNOWN_RETRIEVER")
    }));
}

#[test]
fn reserved_profile_allows_reserved_strategy_references() {
    let registry = FakeStrategyRegistry::new([
        ("GRAPH", StrategyRegistrationStatus::Reserved),
        ("SUMMARY_TREE", StrategyRegistrationStatus::Reserved),
    ]);
    let service = RagConfigService::new(
        FakeRagRepository::with_profiles(vec![profile_record_with_status(
            "reserved-code-graph",
            RagStatus::Reserved,
            Some("v1"),
            json!({
                "retrievers": ["GRAPH"],
                "policies": ["SUMMARY_TREE"]
            }),
        )]),
        registry,
    );

    let validation = service.validate_profile("reserved-code-graph").unwrap();

    assert!(validation.valid, "{validation:?}");
    assert!(
        validation
            .errors
            .iter()
            .all(|error| error.code != "RAG_STRATEGY_RESERVED")
    );
    assert!(validation.strategy_refs.iter().any(|reference| {
        reference.strategy_id == "GRAPH" && reference.status == StrategyRegistrationStatus::Reserved
    }));
    assert!(validation.strategy_refs.iter().any(|reference| {
        reference.strategy_id == "SUMMARY_TREE"
            && reference.status == StrategyRegistrationStatus::Reserved
    }));
}

#[test]
fn enabled_profile_still_rejects_reserved_strategy_references() {
    let service = RagConfigService::new(
        FakeRagRepository::with_profiles(vec![profile_record(
            "enabled-graph",
            Some("v1"),
            json!({ "retrievers": ["GRAPH"] }),
        )]),
        FakeStrategyRegistry::new([("GRAPH", StrategyRegistrationStatus::Reserved)]),
    );

    let validation = service.validate_profile("enabled-graph").unwrap();

    assert!(!validation.valid);
    assert!(validation.errors.iter().any(|error| {
        error.code == "RAG_STRATEGY_RESERVED" && error.strategy_id.as_deref() == Some("GRAPH")
    }));
}

#[test]
fn filter_validation_accepts_declared_fields_and_rejects_invalid_filters() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let accepted = service
        .validate_filters(RagFilterValidationRequest {
            corpus_id: "memory-card".to_string(),
            filters: json!({
                "scopeType": "SPACE",
                "status": ["ACTIVE", "DISABLED"]
            }),
        })
        .unwrap();
    assert!(accepted.valid, "{accepted:?}");
    assert_eq!(
        accepted.normalized_filters["scopeType"],
        json!({"op": "eq", "value": "SPACE"})
    );
    assert_eq!(
        accepted.normalized_filters["status"],
        json!({"op": "in", "value": ["ACTIVE", "DISABLED"]})
    );

    let rejected = service
        .validate_filters(RagFilterValidationRequest {
            corpus_id: "memory-card".to_string(),
            filters: json!({
                "unknown": "x",
                "scopeType": 123,
                "status": { "op": "contains", "value": "ACTIVE" }
            }),
        })
        .unwrap();

    assert!(!rejected.valid);
    assert!(
        rejected
            .field_errors
            .iter()
            .any(|error| { error.field == "unknown" && error.code == "RAG_FILTER_FIELD_UNKNOWN" })
    );
    assert!(
        rejected
            .field_errors
            .iter()
            .any(|error| { error.field == "scopeType" && error.code == "RAG_FILTER_TYPE_INVALID" })
    );
    assert!(rejected.field_errors.iter().any(|error| {
        error.field == "status" && error.code == "RAG_FILTER_OPERATOR_UNSUPPORTED"
    }));
}

#[test]
fn filter_validation_accepts_domain_fields_and_rejects_unknown_or_mistyped_values() {
    let service = RagConfigService::new(
        SqliteRagRepository::connect(":memory:").expect("RAG repository should connect"),
        DefaultRagStrategyRegistry::default(),
    );

    let memory = service
        .validate_filters(RagFilterValidationRequest {
            corpus_id: "memory-card".to_string(),
            filters: json!({
                "memoryType": ["RULE", "DECISION"],
                "reviewStatus": "APPROVED",
                "injectable": true,
                "pinLevel": { "op": "gte", "value": 1 },
                "importanceScore": { "op": "lte", "value": 90 },
                "derivedSourceCode": "chat-run-001"
            }),
        })
        .unwrap();
    assert!(memory.valid, "{memory:?}");
    assert_eq!(
        memory.normalized_filters["memoryType"],
        json!({"op": "in", "value": ["RULE", "DECISION"]})
    );
    assert_eq!(
        memory.normalized_filters["pinLevel"],
        json!({"op": "gte", "value": 1})
    );
    assert_eq!(
        memory.normalized_filters["injectable"],
        json!({"op": "eq", "value": true})
    );

    let project_doc = service
        .validate_filters(RagFilterValidationRequest {
            corpus_id: "project-doc".to_string(),
            filters: json!({
                "repoCode": "REPO-TRON",
                "moduleCode": "MOD-RAG",
                "moduleName": "RAG Backend",
                "documentType": ["ADR", "DESIGN"],
                "path": "docs/rag.md"
            }),
        })
        .unwrap();
    assert!(project_doc.valid, "{project_doc:?}");
    assert_eq!(
        project_doc.normalized_filters["documentType"],
        json!({"op": "in", "value": ["ADR", "DESIGN"]})
    );

    let rejected = service
        .validate_filters(RagFilterValidationRequest {
            corpus_id: "memory-card".to_string(),
            filters: json!({
                "metadata.reviewReason": "manual",
                "confidenceScore": "high",
                "injectable": "yes",
                "pinLevel": { "op": "contains", "value": 1 }
            }),
        })
        .unwrap();

    assert!(!rejected.valid);
    assert!(rejected.field_errors.iter().any(|error| {
        error.field == "metadata.reviewReason" && error.code == "RAG_FILTER_FIELD_UNKNOWN"
    }));
    assert!(rejected.field_errors.iter().any(|error| {
        error.field == "confidenceScore" && error.code == "RAG_FILTER_TYPE_INVALID"
    }));
    assert!(
        rejected.field_errors.iter().any(|error| {
            error.field == "injectable" && error.code == "RAG_FILTER_TYPE_INVALID"
        })
    );
    assert!(rejected.field_errors.iter().any(|error| {
        error.field == "pinLevel" && error.code == "RAG_FILTER_OPERATOR_UNSUPPORTED"
    }));
}

#[test]
fn rag_foundation_diagnostics_boundary_does_not_call_runtime_graph_services() {
    let app_source = fs::read_to_string("src/app.rs").expect("app source should be readable");
    let http_sources = read_rs_sources("src/adapters/inbound/http");
    let rag_application_sources = read_rs_sources("src/application/rag");

    assert!(
        http_sources.contains("/api/v1/rag/search"),
        "RAG search-context-pack now owns the search route"
    );
    assert!(
        http_sources.contains("/api/v1/rag/debug/profile"),
        "RAG diagnostics now owns the debug profile route"
    );
    assert!(
        http_sources.contains("/api/v1/rag/retry/index"),
        "RAG diagnostics now owns the retry index route"
    );
    for forbidden_route in ["/api/v1/rag/graph"] {
        assert!(!app_source.contains(forbidden_route));
        assert!(!http_sources.contains(forbidden_route));
    }

    for forbidden in ["GraphService"] {
        assert!(
            !rag_application_sources.contains(forbidden),
            "RAG foundation diagnostics must not call {forbidden}"
        );
    }
}

#[test]
fn http_rag_handler_uses_facade_instead_of_assembling_rag_internals() {
    let handler =
        fs::read_to_string("src/adapters/inbound/http/rag/handler.rs").expect("handler source");
    let app_source = fs::read_to_string("src/app.rs").expect("app source should be readable");
    let app_startup_source =
        fs::read_to_string("src/app/startup.rs").expect("app startup source should be readable");
    let app_assembly_source = format!("{app_source}\n{app_startup_source}");

    assert!(
        handler.contains("State(rag): State<AppRagService>"),
        "HTTP RAG handlers should receive the unified AppRagService facade from state"
    );
    for forbidden in [
        "SqliteRagRepository",
        "RagIngestService",
        "RagSearchService",
        "RagDiagnosticsService",
        "RagRetryIndexService",
    ] {
        assert!(
            !handler.contains(forbidden),
            "HTTP RAG handler should not assemble {forbidden} directly"
        );
    }
    assert!(
        app_source.contains("pub rag: AppRagService"),
        "AppState should own the unified RAG facade"
    );
    assert!(
        app_assembly_source.contains("fn build_rag_service("),
        "app assembly should be the only RAG SQLite wiring point"
    );
}

#[derive(Clone)]
struct FakeRagRepository {
    profiles: Vec<RagProfileRecord>,
}

impl FakeRagRepository {
    fn with_profiles(profiles: Vec<RagProfileRecord>) -> Self {
        Self { profiles }
    }
}

impl RagConfigRepository for FakeRagRepository {
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        Ok(vec![memory_corpus_record()])
    }

    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
        Ok((corpus_id == "memory-card").then(memory_corpus_record))
    }

    fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileRecord>, RagError> {
        Ok(self
            .profiles
            .iter()
            .filter(|profile| {
                query
                    .profile_type
                    .as_ref()
                    .is_none_or(|kind| profile.profile_type == *kind)
                    && query
                        .status
                        .as_ref()
                        .is_none_or(|status| profile.status == *status)
                    && query.corpus_id.as_ref().is_none_or(|corpus_id| {
                        profile.primary_corpus_id.as_ref() == Some(corpus_id)
                            || profile.corpus_ids.contains(corpus_id)
                    })
            })
            .cloned()
            .collect())
    }

    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagError> {
        Ok(self
            .profiles
            .iter()
            .find(|profile| profile.profile_id == profile_id)
            .cloned())
    }
}

#[derive(Clone)]
struct FakeStrategyRegistry {
    statuses: BTreeMap<String, StrategyRegistrationStatus>,
}

impl FakeStrategyRegistry {
    fn new<const N: usize>(entries: [(&str, StrategyRegistrationStatus); N]) -> Self {
        Self {
            statuses: entries
                .into_iter()
                .map(|(id, status)| (id.to_string(), status))
                .collect(),
        }
    }
}

impl RagStrategyRegistry for FakeStrategyRegistry {
    fn resolve_stage(&self, strategy_id: &str) -> StrategyRegistration {
        self.resolve(strategy_id)
    }

    fn resolve_payload_strategy(&self, strategy_id: &str) -> StrategyRegistration {
        self.resolve(strategy_id)
    }

    fn resolve_retriever(&self, strategy_id: &str) -> StrategyRegistration {
        self.resolve(strategy_id)
    }

    fn resolve_policy(&self, strategy_id: &str) -> StrategyRegistration {
        self.resolve(strategy_id)
    }
}

impl FakeStrategyRegistry {
    fn resolve(&self, strategy_id: &str) -> StrategyRegistration {
        StrategyRegistration {
            strategy_id: strategy_id.to_string(),
            status: self
                .statuses
                .get(strategy_id)
                .cloned()
                .unwrap_or(StrategyRegistrationStatus::Unsupported),
        }
    }
}

fn profile_record(
    profile_id: &str,
    required_source_version: Option<&str>,
    config: serde_json::Value,
) -> RagProfileRecord {
    profile_record_with_status(
        profile_id,
        RagStatus::Enabled,
        required_source_version,
        config,
    )
}

fn profile_record_with_status(
    profile_id: &str,
    status: RagStatus,
    required_source_version: Option<&str>,
    config: serde_json::Value,
) -> RagProfileRecord {
    RagProfileRecord {
        profile_id: profile_id.to_string(),
        profile_type: RagProfileType::Retrieval,
        status,
        primary_corpus_id: Some("memory-card".to_string()),
        corpus_ids: vec!["memory-card".to_string()],
        required_source_version: required_source_version.map(str::to_string),
        config,
        filters: json!({}),
        degrade_policy: "SKIP_VECTOR_ON_UNAVAILABLE".to_string(),
    }
}

fn memory_corpus_record() -> RagCorpusRecord {
    RagCorpusRecord {
        corpus_id: "memory-card".to_string(),
        display_name: "Memory Card".to_string(),
        status: RagStatus::Enabled,
        description: Some("Memory corpus".to_string()),
        capabilities: json!({ "fulltext": true, "vector": true }),
        filter_schema: json!({
            "fields": {
                "scopeType": { "type": "string", "column": "scope_type" },
                "status": { "type": "string", "column": "source_status" }
            }
        }),
        field_mapping: BTreeMap::from([
            ("sourceCode".to_string(), "source_code".to_string()),
            ("scopeType".to_string(), "scope_type".to_string()),
        ]),
        source_field_mapping: BTreeMap::from([
            ("sourceCode".to_string(), "source_code".to_string()),
            ("scopeType".to_string(), "scope_type".to_string()),
        ]),
        text_unit_field_mapping: BTreeMap::new(),
        storage_binding: targeted_sqlite_binding("rag_memory_text_units"),
    }
}

fn targeted_sqlite_binding(table: &str) -> RagStorageBinding {
    RagStorageBinding {
        segment_bindings: BTreeMap::from([
            ("source".to_string(), "primary-store".to_string()),
            ("textUnit".to_string(), "primary-store".to_string()),
            ("fulltext".to_string(), "primary-store".to_string()),
            ("vector".to_string(), "primary-store".to_string()),
            ("diagnostics".to_string(), "diagnostics-store".to_string()),
        ]),
        segment_targets: BTreeMap::from([
            (
                "primary-store".to_string(),
                RagStorageSegmentTarget {
                    id: "primary-store".to_string(),
                    backend: RagStorageBackend::Sqlite,
                    layout: "TABLE".to_string(),
                    table: Some(table.to_string()),
                    namespace: None,
                    node_label: None,
                    code_strategy: None,
                },
            ),
            (
                "diagnostics-store".to_string(),
                RagStorageSegmentTarget {
                    id: "diagnostics-store".to_string(),
                    backend: RagStorageBackend::Sqlite,
                    layout: "TABLE".to_string(),
                    table: Some("rag_diagnostics".to_string()),
                    namespace: None,
                    node_label: None,
                    code_strategy: None,
                },
            ),
        ]),
    }
}

fn read_rs_sources(path: &str) -> String {
    let mut body = String::new();
    for entry in fs::read_dir(path).expect("source directory should be readable") {
        let entry = entry.expect("source entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            body.push_str(&read_rs_sources(
                path.to_str().expect("path should be utf8"),
            ));
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            body.push_str(&fs::read_to_string(&path).expect("rust source file should be readable"));
        }
    }
    body
}
