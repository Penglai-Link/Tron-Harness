use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::domain::rag::{
    RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus, RagStorageBinding,
};

pub trait RagConfigRepository {
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError>;
    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError>;
    fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileRecord>, RagError>;
    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagError>;
}

pub trait RagStrategyRegistry: Clone + Send + Sync + 'static {
    fn resolve_stage(&self, strategy_id: &str) -> StrategyRegistration;
    fn resolve_retriever(&self, strategy_id: &str) -> StrategyRegistration;
    fn resolve_policy(&self, strategy_id: &str) -> StrategyRegistration;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyRegistration {
    pub strategy_id: String,
    pub status: StrategyRegistrationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyRegistrationStatus {
    Enabled,
    Reserved,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RagProfileListQuery {
    pub profile_type: Option<RagProfileType>,
    pub status: Option<RagStatus>,
    pub corpus_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagCorpusView {
    pub corpus_id: String,
    pub display_name: String,
    pub status: RagStatus,
    pub description: Option<String>,
    pub capabilities: Value,
    pub filter_schema: RagFilterSchema,
    pub field_mapping: BTreeMap<String, String>,
    pub storage_binding: RagStorageBinding,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagProfileView {
    pub profile_id: String,
    pub profile_type: RagProfileType,
    pub status: RagStatus,
    pub primary_corpus_id: Option<String>,
    pub corpus_ids: Vec<String>,
    pub required_source_version: Option<String>,
    pub config: Value,
    pub filters: Value,
    pub degrade_policy: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagProfileValidationView {
    pub profile_id: String,
    pub valid: bool,
    pub status: RagStatus,
    pub errors: Vec<RagProfileValidationError>,
    pub strategy_refs: Vec<RagStrategyReferenceView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagProfileValidationError {
    pub code: String,
    pub message: String,
    pub strategy_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagStrategyReferenceView {
    pub kind: String,
    pub strategy_id: String,
    pub status: StrategyRegistrationStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagFilterValidationRequest {
    pub corpus_id: String,
    pub filters: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagFilterValidationView {
    pub valid: bool,
    pub normalized_filters: Value,
    pub field_errors: Vec<RagFilterFieldError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagFilterFieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RagFilterSchema {
    pub fields: BTreeMap<String, RagFilterSchemaField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagFilterSchemaField {
    pub field_type: RagFilterValueType,
    pub column: Option<String>,
    pub operators: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagFilterValueType {
    String,
    Integer,
    Boolean,
}

pub struct RagService<R, S> {
    repository: R,
    strategy_registry: S,
}

impl<R, S> RagService<R, S>
where
    R: RagConfigRepository,
    S: RagStrategyRegistry,
{
    pub fn new(repository: R, strategy_registry: S) -> Self {
        Self {
            repository,
            strategy_registry,
        }
    }

    pub fn list_corpora(&self) -> Result<Vec<RagCorpusView>, RagError> {
        self.repository
            .list_corpora()?
            .into_iter()
            .map(project_corpus)
            .collect()
    }

    pub fn get_corpus(&self, corpus_id: &str) -> Result<RagCorpusView, RagError> {
        self.repository
            .get_corpus(corpus_id)?
            .ok_or(RagError::CorpusNotFound)
            .and_then(project_corpus)
    }

    pub fn get_storage_binding(&self, corpus_id: &str) -> Result<RagStorageBinding, RagError> {
        Ok(self.get_corpus(corpus_id)?.storage_binding)
    }

    pub fn list_profiles(
        &self,
        query: RagProfileListQuery,
    ) -> Result<Vec<RagProfileView>, RagError> {
        Ok(self
            .repository
            .list_profiles(query)?
            .into_iter()
            .map(project_profile)
            .collect())
    }

    pub fn get_profile(&self, profile_id: &str) -> Result<RagProfileView, RagError> {
        self.repository
            .get_profile(profile_id)?
            .ok_or(RagError::ProfileNotFound)
            .map(project_profile)
    }

    pub fn validate_profile(&self, profile_id: &str) -> Result<RagProfileValidationView, RagError> {
        let profile = self
            .repository
            .get_profile(profile_id)?
            .ok_or(RagError::ProfileNotFound)?;
        let mut errors = Vec::new();
        let mut strategy_refs = Vec::new();

        if profile.profile_type == RagProfileType::Retrieval {
            match profile.required_source_version.as_deref().map(str::trim) {
                Some(version) if !version.is_empty() && !version.eq_ignore_ascii_case("latest") => {
                }
                Some(_) => errors.push(profile_error(
                    "RAG_PROFILE_SOURCE_VERSION_REQUIRED",
                    "Retrieval Profile 不接受 latest 作为 requiredSourceVersion",
                    None,
                )),
                None => errors.push(profile_error(
                    "RAG_PROFILE_SOURCE_VERSION_REQUIRED",
                    "Retrieval Profile 缺少 requiredSourceVersion",
                    None,
                )),
            }
        }

        for (kind, strategy_id) in collect_strategy_refs(&profile.config) {
            let registration = match kind.as_str() {
                "stage" => self.strategy_registry.resolve_stage(&strategy_id),
                "retriever" => self.strategy_registry.resolve_retriever(&strategy_id),
                "policy" => self.strategy_registry.resolve_policy(&strategy_id),
                _ => continue,
            };
            match registration.status {
                StrategyRegistrationStatus::Enabled => {}
                StrategyRegistrationStatus::Reserved if profile.status == RagStatus::Reserved => {}
                StrategyRegistrationStatus::Reserved => {
                    errors.push(profile_error(
                        "RAG_STRATEGY_RESERVED",
                        "enabled Profile 引用了 reserved strategy",
                        Some(strategy_id.clone()),
                    ));
                }
                StrategyRegistrationStatus::Unsupported => errors.push(profile_error(
                    "RAG_STRATEGY_UNREGISTERED",
                    "Profile 引用了未注册 strategy",
                    Some(strategy_id.clone()),
                )),
            }
            strategy_refs.push(RagStrategyReferenceView {
                kind,
                strategy_id,
                status: registration.status,
            });
        }

        Ok(RagProfileValidationView {
            profile_id: profile.profile_id,
            valid: errors.is_empty(),
            status: profile.status,
            errors,
            strategy_refs,
        })
    }

    pub fn validate_filters(
        &self,
        request: RagFilterValidationRequest,
    ) -> Result<RagFilterValidationView, RagError> {
        let corpus = self.get_corpus(&request.corpus_id)?;
        let filters = request
            .filters
            .as_object()
            .ok_or(RagError::StorageBindingInvalid)?;
        let mut normalized = serde_json::Map::new();
        let mut field_errors = Vec::new();

        for (field, raw_value) in filters {
            let Some(schema_field) = corpus.filter_schema.fields.get(field) else {
                field_errors.push(filter_error(
                    field,
                    "RAG_FILTER_FIELD_UNKNOWN",
                    "filter 字段未声明",
                ));
                continue;
            };
            let (operator, value) = extract_filter_operator(raw_value);
            if !schema_field.operators.contains(operator) {
                field_errors.push(filter_error(
                    field,
                    "RAG_FILTER_OPERATOR_UNSUPPORTED",
                    "filter 操作符不受支持",
                ));
                continue;
            }
            if !filter_value_matches(&schema_field.field_type, operator, value) {
                field_errors.push(filter_error(
                    field,
                    "RAG_FILTER_TYPE_INVALID",
                    "filter 值类型不匹配",
                ));
                continue;
            }
            normalized.insert(
                field.clone(),
                json!({ "op": operator, "value": value.clone() }),
            );
        }

        Ok(RagFilterValidationView {
            valid: field_errors.is_empty(),
            normalized_filters: Value::Object(normalized),
            field_errors,
        })
    }
}

#[derive(Clone, Default)]
pub struct DefaultRagStrategyRegistry;

impl RagStrategyRegistry for DefaultRagStrategyRegistry {
    fn resolve_stage(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(strategy_id, &["INGEST_CHUNK"])
    }

    fn resolve_retriever(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(strategy_id, &["FULLTEXT", "VECTOR"])
    }

    fn resolve_policy(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(
            strategy_id,
            &[
                "SKIP_VECTOR_ON_UNAVAILABLE",
                "DEDUP_BY_SOURCE",
                "HYDRATE_TEXT_UNIT",
                "CONTEXT_PACK_BASIC",
            ],
        )
    }
}

fn default_registration(strategy_id: &str, enabled_ids: &[&str]) -> StrategyRegistration {
    let status = if enabled_ids.contains(&strategy_id) {
        StrategyRegistrationStatus::Enabled
    } else if matches!(strategy_id, "GRAPH" | "SUMMARY_TREE") {
        StrategyRegistrationStatus::Reserved
    } else {
        StrategyRegistrationStatus::Unsupported
    };
    StrategyRegistration {
        strategy_id: strategy_id.to_string(),
        status,
    }
}

fn project_corpus(record: RagCorpusRecord) -> Result<RagCorpusView, RagError> {
    validate_storage_binding(&record.storage_binding, &record.status)?;
    Ok(RagCorpusView {
        corpus_id: record.corpus_id,
        display_name: record.display_name,
        status: record.status,
        description: record.description,
        capabilities: record.capabilities,
        filter_schema: parse_filter_schema(&record.filter_schema),
        field_mapping: record.field_mapping,
        storage_binding: record.storage_binding,
    })
}

fn project_profile(record: RagProfileRecord) -> RagProfileView {
    RagProfileView {
        profile_id: record.profile_id,
        profile_type: record.profile_type,
        status: record.status,
        primary_corpus_id: record.primary_corpus_id,
        corpus_ids: record.corpus_ids,
        required_source_version: record.required_source_version,
        config: record.config,
        filters: record.filters,
        degrade_policy: record.degrade_policy,
    }
}

fn validate_storage_binding(
    binding: &RagStorageBinding,
    status: &RagStatus,
) -> Result<(), RagError> {
    if *status == RagStatus::Reserved {
        return Ok(());
    }
    for table_name in [
        binding.source_table.as_deref(),
        binding.text_unit_table.as_deref(),
        binding.diagnostics_table.as_deref(),
    ] {
        let Some(table_name) = table_name else {
            return Err(RagError::StorageBindingInvalid);
        };
        if !is_valid_table_name(table_name) {
            return Err(RagError::StorageBindingInvalid);
        }
    }
    Ok(())
}

fn is_valid_table_name(table_name: &str) -> bool {
    table_name.starts_with("rag_")
        && table_name.chars().all(|character| {
            character.is_ascii_lowercase() || character == '_' || character.is_ascii_digit()
        })
}

fn parse_filter_schema(value: &Value) -> RagFilterSchema {
    let mut schema = RagFilterSchema::default();
    let Some(fields) = value.get("fields").and_then(Value::as_object) else {
        return schema;
    };

    for (name, raw_field) in fields {
        let field_type = match raw_field
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("string")
        {
            "integer" | "number" => RagFilterValueType::Integer,
            "boolean" => RagFilterValueType::Boolean,
            _ => RagFilterValueType::String,
        };
        let mut operators: BTreeSet<String> = raw_field
            .get("operators")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_else(|| BTreeSet::from(["eq".to_string(), "in".to_string()]));
        if operators.is_empty() {
            operators.extend(["eq".to_string(), "in".to_string()]);
        }
        schema.fields.insert(
            name.clone(),
            RagFilterSchemaField {
                field_type,
                column: raw_field
                    .get("column")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                operators,
            },
        );
    }
    schema
}

fn extract_filter_operator(raw_value: &Value) -> (&str, &Value) {
    if let Some(object) = raw_value.as_object()
        && let (Some(operator), Some(value)) = (
            object.get("op").and_then(Value::as_str),
            object.get("value"),
        )
    {
        return (operator, value);
    }
    if raw_value.is_array() {
        ("in", raw_value)
    } else {
        ("eq", raw_value)
    }
}

fn filter_value_matches(value_type: &RagFilterValueType, operator: &str, value: &Value) -> bool {
    if operator == "in" {
        return value.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|item| single_value_matches(value_type, item))
        });
    }
    single_value_matches(value_type, value)
}

fn single_value_matches(value_type: &RagFilterValueType, value: &Value) -> bool {
    match value_type {
        RagFilterValueType::String => value.is_string(),
        RagFilterValueType::Integer => value.as_i64().is_some(),
        RagFilterValueType::Boolean => value.is_boolean(),
    }
}

fn collect_strategy_refs(config: &Value) -> Vec<(String, String)> {
    let mut refs = Vec::new();
    collect_strategy_refs_inner(config, None, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

fn collect_strategy_refs_inner(
    value: &Value,
    current_kind: Option<&'static str>,
    refs: &mut Vec<(String, String)>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let kind = strategy_kind_for_key(key).or(current_kind);
                collect_strategy_refs_inner(value, kind, refs);
            }
        }
        Value::Array(items) => {
            for value in items {
                collect_strategy_refs_inner(value, current_kind, refs);
            }
        }
        Value::String(strategy_id) => {
            if let Some(kind) = current_kind
                && looks_like_strategy_id(strategy_id)
            {
                refs.push((kind.to_string(), strategy_id.clone()));
            }
        }
        _ => {}
    }
}

fn strategy_kind_for_key(key: &str) -> Option<&'static str> {
    match key {
        "stage" | "stages" | "stageId" | "stageIds" => Some("stage"),
        "retriever" | "retrievers" | "retrieverId" | "retrieverIds" | "route" | "routes" => {
            Some("retriever")
        }
        "policy" | "policies" | "degradePolicy" | "fusion" | "dedup" | "rerank" | "hydrate"
        | "context" => Some("policy"),
        _ => None,
    }
}

fn looks_like_strategy_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().all(|character| {
            character.is_ascii_uppercase() || character == '_' || character.is_ascii_digit()
        })
}

fn profile_error(
    code: &str,
    message: &str,
    strategy_id: Option<String>,
) -> RagProfileValidationError {
    RagProfileValidationError {
        code: code.to_string(),
        message: message.to_string(),
        strategy_id,
    }
}

fn filter_error(field: &str, code: &str, message: &str) -> RagFilterFieldError {
    RagFilterFieldError {
        field: field.to_string(),
        code: code.to_string(),
        message: message.to_string(),
    }
}
