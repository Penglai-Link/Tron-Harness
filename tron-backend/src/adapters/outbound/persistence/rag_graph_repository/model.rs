use std::{
    collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use serde_json::{Map, Value, json};

use crate::{
    application::rag::{RagPersistIngestCommand, RagRetryIndexRequest, RagSearchRow},
    domain::{
        graph::{
            GraphError, GraphFilterOperator, GraphNode, GraphPropertyFilter, GraphPropertyValue,
            GraphScopeCommand,
        },
        rag::{RagCorpusRecord, RagError},
    },
};

use super::GraphBackedRagRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GraphRuntimeTarget {
    pub(super) id: String,
    pub(super) namespace: String,
    pub(super) node_label: String,
    pub(super) code_strategy: Option<String>,
}

impl GraphBackedRagRepository {
    pub(super) fn resolve_graph_scope_for_namespace(
        &self,
        namespace: &str,
        workspace_code: &str,
        scope_type: &str,
        scope_code: &str,
    ) -> Result<crate::domain::graph::ResolvedGraphScope, RagError> {
        let command = match scope_type {
            "USER" => GraphScopeCommand {
                namespace: namespace.to_string(),
                workspace_code: workspace_code.to_string(),
                app_code: None,
                user_code: Some(scope_code.to_string()),
            },
            "AGENT" => GraphScopeCommand {
                namespace: namespace.to_string(),
                workspace_code: workspace_code.to_string(),
                app_code: Some(scope_code.to_string()),
                user_code: None,
            },
            "SPACE" => GraphScopeCommand {
                namespace: namespace.to_string(),
                workspace_code: scope_code.to_string(),
                app_code: None,
                user_code: None,
            },
            _ => return Err(RagError::StorageBindingInvalid),
        };
        self.scope_resolver
            .resolve_graph_scope(command)
            .map_err(map_graph_error)
    }

    pub(super) fn resolve_graph_scope(
        &self,
        corpus: &RagCorpusRecord,
        workspace_code: &str,
        scope_type: &str,
        scope_code: &str,
    ) -> Result<crate::domain::graph::ResolvedGraphScope, RagError> {
        let target = default_graph_target(corpus)?;
        self.resolve_graph_scope_for_namespace(
            &target.namespace,
            workspace_code,
            scope_type,
            scope_code,
        )
    }

    pub(super) fn find_graph_main_node(
        &self,
        corpus: &RagCorpusRecord,
        request: &RagRetryIndexRequest,
    ) -> Result<Option<GraphNode>, RagError> {
        let scope = self.resolve_graph_scope(
            corpus,
            &request.workspace_code,
            &request.scope_type,
            &request.scope_code,
        )?;
        let node_code = graph_main_node_code_parts(
            &request.corpus_id,
            &request.source_code,
            &request.source_version,
            &request.scope_type,
            &request.scope_code,
        );
        if let Some(node) = self
            .graph_store
            .get_node(&scope, &node_code)
            .map_err(map_graph_error)?
        {
            return Ok(Some(node));
        }
        let legacy_node_code = legacy_graph_skill_node_code_parts(
            &request.corpus_id,
            &request.source_code,
            &request.source_version,
            &request.scope_type,
            &request.scope_code,
        );
        if legacy_node_code == node_code {
            return Ok(None);
        }
        self.graph_store
            .get_node(&scope, &legacy_node_code)
            .map_err(map_graph_error)
    }
}

pub(super) fn has_graph_targets(corpus: &RagCorpusRecord) -> bool {
    corpus
        .storage_binding
        .segment_bindings
        .iter()
        .filter(|(segment, _)| segment.as_str() != "diagnostics")
        .any(|(_, target_id)| {
            corpus
                .storage_binding
                .segment_targets
                .get(target_id)
                .is_some_and(|target| target.backend.is_graph())
        })
}

pub(super) fn graph_target_for_retriever(
    corpus: &RagCorpusRecord,
    retriever: &str,
) -> Result<GraphRuntimeTarget, RagError> {
    let segment = graph_segment_for_retriever(retriever)?;
    optional_graph_target_for_segment(corpus, segment)?
        .or_else(|| {
            if segment == "fulltext" {
                optional_graph_target_for_segment(corpus, "textUnit")
                    .ok()
                    .flatten()
            } else {
                None
            }
        })
        .ok_or(RagError::SearchRouteUnavailable)
}

pub(super) fn graph_segment_for_retriever(retriever: &str) -> Result<&'static str, RagError> {
    match retriever {
        "FULLTEXT_GRAPH_SINGLE_NODE_V1" => Ok("fulltext"),
        "VECTOR_GRAPH_SINGLE_NODE_V1" => Ok("vector"),
        "TREE_FULLTEXT_V1" | "TREE_VECTOR_V1" => Ok("tree"),
        _ => Err(RagError::SearchRouteUnavailable),
    }
}

pub(super) fn optional_graph_target_for_segment(
    corpus: &RagCorpusRecord,
    segment: &str,
) -> Result<Option<GraphRuntimeTarget>, RagError> {
    let Some(target_id) = corpus.storage_binding.segment_bindings.get(segment) else {
        return Ok(None);
    };
    let Some(target) = corpus.storage_binding.segment_targets.get(target_id) else {
        return Err(RagError::StorageBindingInvalid);
    };
    if !target.backend.is_graph() {
        return Ok(None);
    }
    let namespace = target
        .namespace
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or(RagError::StorageBindingInvalid)?;
    let node_label = target
        .node_label
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or(RagError::StorageBindingInvalid)?;
    Ok(Some(GraphRuntimeTarget {
        id: target.id.clone(),
        namespace,
        node_label,
        code_strategy: target.code_strategy.clone(),
    }))
}

fn default_graph_target(corpus: &RagCorpusRecord) -> Result<GraphRuntimeTarget, RagError> {
    for segment in ["textUnit", "fulltext", "vector", "tree", "source"] {
        if let Some(target) = optional_graph_target_for_segment(corpus, segment)? {
            return Ok(target);
        }
    }
    Err(RagError::StorageBindingInvalid)
}

pub(super) fn node_label(corpus: &RagCorpusRecord) -> Result<String, RagError> {
    optional_graph_target_for_segment(corpus, "textUnit")?
        .or_else(|| {
            optional_graph_target_for_segment(corpus, "fulltext")
                .ok()
                .flatten()
        })
        .or_else(|| {
            optional_graph_target_for_segment(corpus, "vector")
                .ok()
                .flatten()
        })
        .map(|target| target.node_label)
        .ok_or(RagError::StorageBindingInvalid)
}

pub(super) fn graph_hit_to_row(
    corpus: &RagCorpusRecord,
    node: GraphNode,
    score: Option<f64>,
    segment: &str,
) -> RagSearchRow {
    let props = &node.properties;
    let metadata = prop_text(props, "sourceMetadataJson")
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_else(|| json!({}));
    let embedding = prop_text(props, "embeddingJson")
        .and_then(|value| serde_json::from_str::<Vec<f32>>(value).ok());
    RagSearchRow {
        corpus_id: corpus.corpus_id.clone(),
        source_code: prop_text(props, "sourceCode")
            .unwrap_or_default()
            .to_string(),
        source_version: prop_text(props, "sourceVersion")
            .unwrap_or_default()
            .to_string(),
        target_code: node.node_code,
        scope_type: prop_text(props, "scopeType")
            .unwrap_or_default()
            .to_string(),
        scope_code: prop_text(props, "scopeCode")
            .unwrap_or_default()
            .to_string(),
        title: prop_text(props, "title").map(ToOwned::to_owned),
        text: prop_text(props, "text").unwrap_or_default().to_string(),
        search_text: prop_text(props, "searchText")
            .unwrap_or_default()
            .to_string(),
        heading_path: Vec::new(),
        ordinal: 0,
        token_count: prop_i64(props, "tokenCount").unwrap_or(1),
        embedding,
        storage_score: score,
        source_metadata: metadata,
        text_metadata: json!({}),
        field_values: graph_field_values(corpus, props, segment),
    }
}

pub(super) fn append_graph_filters(
    filters: &mut Vec<GraphPropertyFilter>,
    corpus: &RagCorpusRecord,
    raw: &Value,
    segment: &str,
) {
    let Some(object) = raw.as_object() else {
        return;
    };
    for (field, value) in object {
        if matches!(field.as_str(), "sourceVersion" | "ragWorkspaceCode") {
            continue;
        }
        let value = value.get("value").unwrap_or(value);
        let Some(text) = value.as_str() else {
            continue;
        };
        let property_name =
            graph_property_name_for_logical(corpus, segment, field).unwrap_or(field);
        filters.push(graph_filter(property_name, text));
    }
}

pub(super) fn graph_filter(property_name: &str, value: &str) -> GraphPropertyFilter {
    GraphPropertyFilter {
        property_name: property_name.to_string(),
        operator: GraphFilterOperator::Equals,
        value: GraphPropertyValue::Text(value.to_string()),
    }
}

pub(super) fn fold_text_units(command: &RagPersistIngestCommand) -> String {
    command
        .text_units
        .iter()
        .map(|unit| unit.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(super) fn graph_main_node_code(command: &RagPersistIngestCommand) -> String {
    graph_main_node_code_parts(
        &command.corpus_id,
        &command.source_code,
        &command.source_version,
        &command.scope.scope_type,
        &command.scope.scope_code,
    )
}

pub(super) fn legacy_graph_skill_node_code(command: &RagPersistIngestCommand) -> String {
    legacy_graph_skill_node_code_parts(
        &command.corpus_id,
        &command.source_code,
        &command.source_version,
        &command.scope.scope_type,
        &command.scope.scope_code,
    )
}

pub(super) fn graph_segment_node_code(
    segment: &str,
    command: &RagPersistIngestCommand,
    text_unit_code: Option<&str>,
) -> String {
    graph_segment_node_code_parts(
        segment,
        &command.corpus_id,
        &command.source_code,
        &command.source_version,
        &command.scope.scope_type,
        &command.scope.scope_code,
        text_unit_code,
    )
}

pub(super) fn graph_segment_node_code_parts(
    segment: &str,
    corpus_id: &str,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
    text_unit_code: Option<&str>,
) -> String {
    let prefix = format!(
        "RAG_{}",
        segment
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
    );
    let mut parts = vec![
        corpus_id,
        source_code,
        source_version,
        scope_type,
        scope_code,
    ];
    if let Some(text_unit_code) = text_unit_code {
        parts.push(text_unit_code);
    }
    stable_graph_code(&prefix, &parts)
}

pub(super) fn graph_main_node_code_parts(
    corpus_id: &str,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> String {
    stable_graph_code(
        "RAG_GRAPH_NODE",
        &[
            corpus_id,
            source_code,
            source_version,
            scope_type,
            scope_code,
        ],
    )
}

pub(super) fn legacy_graph_skill_node_code_parts(
    corpus_id: &str,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> String {
    stable_graph_code(
        "RAG_SKILL_GRAPH",
        &[
            corpus_id,
            source_code,
            source_version,
            scope_type,
            scope_code,
        ],
    )
}

pub(super) fn stable_graph_code(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    format!("{prefix}_{:016X}", hasher.finish())
}

pub(super) fn normalize_related_code(value: &str) -> Result<String, RagError> {
    let normalized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if normalized.is_empty() {
        Err(RagError::IngestRequestInvalid)
    } else {
        Ok(normalized)
    }
}

pub(super) fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, RagError> {
    optional_str(value, field).ok_or(RagError::IngestRequestInvalid)
}

pub(super) fn optional_str<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn push_text_prop(
    properties: &mut BTreeMap<String, GraphPropertyValue>,
    name: &str,
    value: &str,
) {
    properties.insert(
        name.to_string(),
        GraphPropertyValue::Text(value.to_string()),
    );
}

pub(super) fn prop_text<'a>(
    properties: &'a BTreeMap<String, GraphPropertyValue>,
    name: &str,
) -> Option<&'a str> {
    match properties.get(name) {
        Some(GraphPropertyValue::Text(value)) => Some(value.as_str()),
        _ => None,
    }
}

pub(super) fn prop_i64(
    properties: &BTreeMap<String, GraphPropertyValue>,
    name: &str,
) -> Option<i64> {
    match properties.get(name) {
        Some(GraphPropertyValue::Integer(value)) => Some(*value),
        _ => None,
    }
}

pub(super) fn graph_text(node: &GraphNode) -> String {
    prop_text(&node.properties, "text")
        .or(node.full_text.as_deref())
        .unwrap_or_default()
        .to_string()
}

pub(super) fn graph_node_json(node: &GraphNode) -> Value {
    json!({
        "nodeCode": node.node_code,
        "labels": node.labels,
        "title": prop_text(&node.properties, "title"),
        "relatedKind": prop_text(&node.properties, "relatedKind"),
        "text": prop_text(&node.properties, "text"),
        "properties": graph_properties_json(&node.properties)
    })
}

fn graph_field_values(
    corpus: &RagCorpusRecord,
    properties: &BTreeMap<String, GraphPropertyValue>,
    segment: &str,
) -> Map<String, Value> {
    let logical_names = graph_logical_field_names(corpus);
    Map::from_iter(logical_names.into_iter().filter_map(|logical_name| {
        let property_name = graph_property_name_for_logical(corpus, segment, &logical_name)?;
        properties
            .get(property_name)
            .map(|value| (logical_name, graph_property_json(value)))
    }))
}

pub(super) fn graph_property_name_for_logical<'a>(
    corpus: &'a RagCorpusRecord,
    segment: &str,
    logical_name: &str,
) -> Option<&'a str> {
    let primary = match segment {
        "source" => non_empty_mapping(&corpus.source_field_mapping),
        "textUnit" | "fulltext" | "vector" => non_empty_mapping(&corpus.text_unit_field_mapping),
        "tree" => non_empty_mapping(&corpus.source_field_mapping),
        _ => None,
    };
    primary
        .and_then(|mapping| mapping.get(logical_name))
        .or_else(|| {
            if segment != "source" {
                corpus.source_field_mapping.get(logical_name)
            } else {
                None
            }
        })
        .or_else(|| corpus.field_mapping.get(logical_name))
        .map(String::as_str)
}

fn non_empty_mapping(mapping: &BTreeMap<String, String>) -> Option<&BTreeMap<String, String>> {
    (!mapping.is_empty()).then_some(mapping)
}

fn graph_logical_field_names(corpus: &RagCorpusRecord) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.extend(corpus.field_mapping.keys().cloned());
    names.extend(corpus.source_field_mapping.keys().cloned());
    names.extend(corpus.text_unit_field_mapping.keys().cloned());
    names
}

pub(super) fn graph_properties_json(properties: &BTreeMap<String, GraphPropertyValue>) -> Value {
    Value::Object(Map::from_iter(
        properties
            .iter()
            .map(|(name, value)| (name.clone(), graph_property_json(value))),
    ))
}

fn graph_property_json(value: &GraphPropertyValue) -> Value {
    match value {
        GraphPropertyValue::Text(value) => json!(value),
        GraphPropertyValue::Integer(value) => json!(value),
        GraphPropertyValue::Decimal(value) => json!(value),
        GraphPropertyValue::Boolean(value) => json!(value),
    }
}

pub(super) fn decode_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

pub(super) fn json_string<T: serde::Serialize>(value: &T) -> Result<String, RagError> {
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}

pub(super) fn map_graph_error(error: GraphError) -> RagError {
    match error {
        GraphError::MissingNode { .. } => RagError::SourceNotFound,
        GraphError::InvalidGraphCode
        | GraphError::LabelRequired
        | GraphError::RelationshipRequired
        | GraphError::ReservedPropertyName
        | GraphError::InvalidTraversalQuery
        | GraphError::EmptySearchQuery => RagError::StorageBindingInvalid,
        _ => RagError::StorageFailure,
    }
}
