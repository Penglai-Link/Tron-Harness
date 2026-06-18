use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::domain::rag::{
    RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus, RagStorageBackend,
    RagStorageBinding, RagStorageSegmentTarget,
};

use super::{
    RagConfigRepository, RagProfileListQuery, RagStrategyRegistry, StrategyRegistrationStatus,
    compile_ingest_profile, compile_retrieval_profile,
    pipeline::{
        CompiledRagPipelineNode, RagPipelineCatalogView, RagPipelineEdgeInput, RagPipelineInput,
        RagPipelineNodeInput, execution_stage_for_stage_type, pipeline_catalog,
        stage_type_for_execution_stage, validate_pipeline,
    },
};

const DEFAULT_INGEST_STAGE_ORDER: [&str; 10] = [
    "validateSource",
    "preparePayload",
    "resolveSource",
    "parseDocument",
    "splitText",
    "extractTree",
    "fulltextIndex",
    "vectorIndex",
    "persistTree",
    "persistSource",
];

const SUPPORTED_CONTEXT_STRATEGIES: [&str; 4] = [
    "RESULT_LIST",
    "TOKEN_BUDGET_PACK",
    "MEMORY_CONTEXT_PACK",
    "DOC_QA_CONTEXT_PACK",
];

const SOURCE_CONTRACT_FIELDS: [(&str, &str); 8] = [
    ("sourceCode", "source_code"),
    ("sourceVersion", "source_version"),
    ("sourceType", "source_type"),
    ("scopeType", "scope_type"),
    ("scopeCode", "scope_code"),
    ("status", "status"),
    ("title", "title"),
    ("metadata", "metadata"),
];

const TEXT_UNIT_CONTRACT_FIELDS: [(&str, &str); 6] = [
    ("sourceCode", "source_code"),
    ("sourceVersion", "source_version"),
    ("scopeType", "scope_type"),
    ("scopeCode", "scope_code"),
    ("text", "text"),
    ("textUnitStatus", "status"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct RagCorpusManagementRecord {
    pub record: RagCorpusRecord,
}

pub trait RagManagementRepository {
    fn get_managed_corpus(
        &self,
        corpus_id: &str,
    ) -> Result<Option<RagCorpusManagementRecord>, RagError>;

    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError>;

    fn save_profile(&self, record: &RagProfileRecord) -> Result<(), RagError>;
}

impl<T> RagManagementRepository for Arc<Mutex<T>>
where
    T: RagManagementRepository,
{
    fn get_managed_corpus(
        &self,
        corpus_id: &str,
    ) -> Result<Option<RagCorpusManagementRecord>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .get_managed_corpus(corpus_id)
    }

    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .save_managed_corpus(record)
    }

    fn save_profile(&self, record: &RagProfileRecord) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .save_profile(record)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagMetadataGroup {
    pub name: String,
    #[serde(default)]
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusCapabilitiesPayload {
    pub fulltext: bool,
    pub vector: bool,
    pub graph: bool,
    pub reserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagFieldMappingPayload {
    #[serde(default)]
    pub source: BTreeMap<String, String>,
    #[serde(default)]
    pub text_unit: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagStorageTargetPayload {
    pub id: String,
    pub backend: String,
    pub layout: String,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub node_label: Option<String>,
    #[serde(default)]
    pub code_strategy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagStorageBindingPayload {
    pub mode: String,
    #[serde(default)]
    pub targets: Vec<RagStorageTargetPayload>,
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusManagementInput {
    pub corpus_id: String,
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    pub capabilities: RagCorpusCapabilitiesPayload,
    #[serde(default)]
    pub metadata_groups: Option<Vec<RagMetadataGroup>>,
    #[serde(default)]
    pub filter_schema: Value,
    pub field_mapping: RagFieldMappingPayload,
    pub storage_binding: RagStorageBindingPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagValidationIssueView {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRuntimeProjectionView {
    pub compatible: bool,
    pub storage_binding: RagStorageBindingPayload,
    #[serde(default)]
    pub issues: Vec<RagValidationIssueView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusManagementListItem {
    pub corpus_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: String,
    pub capabilities: RagCorpusCapabilitiesPayload,
    #[serde(default)]
    pub metadata_groups: Vec<RagMetadataGroup>,
    pub storage_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingest_profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusManagementView {
    pub corpus_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: String,
    pub capabilities: RagCorpusCapabilitiesPayload,
    #[serde(default)]
    pub metadata_groups: Vec<RagMetadataGroup>,
    pub filter_schema: Value,
    pub field_mapping: RagFieldMappingPayload,
    pub storage_binding: RagStorageBindingPayload,
    pub runtime_projection: RagRuntimeProjectionView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagContractFieldDescriptor {
    pub logical_name: String,
    pub column: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagContractFieldCatalog {
    pub source: Vec<RagContractFieldDescriptor>,
    pub text_unit: Vec<RagContractFieldDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusCatalogView {
    pub storage_backends: Vec<String>,
    pub code_strategies: Vec<String>,
    pub contract_fields: RagContractFieldCatalog,
    pub filter_field_types: Vec<String>,
    pub filter_operators: Vec<String>,
    pub capability_rules: Vec<RagValidationIssueView>,
    pub compatibility_rules: Vec<RagValidationIssueView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusVerifyView {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<RagValidationIssueView>,
    pub runtime_projection: RagRuntimeProjectionView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineStagePayload {
    pub id: String,
    pub enabled: bool,
    pub strategy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineStageInput {
    pub id: String,
    pub enabled: bool,
    pub strategy_id: String,
    #[serde(default)]
    pub config: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalRoutePayload {
    pub route_id: String,
    pub corpus_id: String,
    pub enabled: bool,
    pub route_type: String,
    pub strategy_id: String,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub filters: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalFilterPayload {
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagProfileManagementInput {
    pub profile_id: String,
    pub profile_type: String,
    pub status: String,
    #[serde(default)]
    pub description: String,
    pub primary_corpus_id: String,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(default)]
    pub required_source_version: Option<String>,
    #[serde(default)]
    pub pipeline: Option<RagPipelineInput>,
    #[serde(default)]
    pub pipeline_stages: Vec<RagPipelineStageInput>,
    #[serde(default)]
    pub embedding_model_code: Option<String>,
    #[serde(default)]
    pub embedding_version: Option<String>,
    #[serde(default)]
    pub graph_write_policy: Option<Value>,
    #[serde(default)]
    pub source_input_policy: Option<Value>,
    #[serde(default)]
    pub routes: Vec<RagRetrievalRoutePayload>,
    #[serde(default)]
    pub filters: Vec<RagRetrievalFilterPayload>,
    #[serde(default)]
    pub fusion_policy: Option<String>,
    #[serde(default)]
    pub dedup_policy: Option<String>,
    #[serde(default)]
    pub topic_policy: Option<String>,
    #[serde(default)]
    pub graph_expansion_policy: Option<String>,
    #[serde(default)]
    pub context_strategies: Vec<String>,
    #[serde(default)]
    pub rerank_enabled: Option<bool>,
    #[serde(default)]
    pub rerank_model_code: Option<String>,
    #[serde(default)]
    pub degrade_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagProfileManagementListItem {
    pub profile_id: String,
    pub profile_type: String,
    pub status: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_corpus_id: Option<String>,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_source_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagProfileManagementView {
    pub profile_id: String,
    pub profile_type: String,
    pub status: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_corpus_id: Option<String>,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_source_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<RagPipelineInput>,
    #[serde(default)]
    pub pipeline_stages: Vec<RagPipelineStagePayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_write_policy: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_input_policy: Option<Value>,
    #[serde(default)]
    pub routes: Vec<RagRetrievalRoutePayload>,
    #[serde(default)]
    pub filters: Vec<RagRetrievalFilterPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fusion_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_expansion_policy: Option<String>,
    #[serde(default)]
    pub context_strategies: Vec<String>,
    pub rerank_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_model_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degrade_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusOptionView {
    pub corpus_id: String,
    pub display_name: String,
    pub status: String,
    pub capabilities: RagCorpusCapabilitiesPayload,
    pub storage_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalRouteTypeOptionView {
    pub corpus_id: String,
    pub route_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalStrategyOptionView {
    pub corpus_id: String,
    pub route_type: String,
    pub strategy_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagProfileCatalogView {
    pub corpora: Vec<RagCorpusOptionView>,
    pub ingest_stage_definitions: Vec<String>,
    pub ingest_stage_strategies: BTreeMap<String, Vec<String>>,
    pub retrieval_route_types: Vec<RagRetrievalRouteTypeOptionView>,
    pub retrieval_strategies: Vec<RagRetrievalStrategyOptionView>,
    pub fusion_policy_types: Vec<String>,
    pub dedup_policy_types: Vec<String>,
    pub topic_policy_types: Vec<String>,
    pub graph_expansion_policies: Vec<String>,
    pub context_strategies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagProfileVerifyView {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<RagValidationIssueView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagFilterValidateInput {
    pub corpus_id: String,
    #[serde(default)]
    pub filters: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagFilterValidateView {
    pub valid: bool,
    pub normalized_filters: Value,
    #[serde(default)]
    pub field_errors: Vec<RagValidationIssueView>,
}

pub struct RagManagementService<R, S> {
    repository: R,
    strategy_registry: S,
}

