use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    application::rag::{RagPersistIngestCommand, RagTextUnitWrite, RagTreeNodeWrite},
    domain::{
        graph::GraphPropertyValue,
        rag::{RagCorpusRecord, RagError},
    },
};

use super::super::model::{
    decode_embedding, graph_property_name_for_logical, json_string, push_text_prop,
};

pub(super) fn source_properties(
    corpus: &RagCorpusRecord,
    command: &RagPersistIngestCommand,
    node_code: &str,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = source_base_properties(command, node_code)?;
    push_domain_field_properties(corpus, "source", command, &mut properties)?;
    Ok(properties)
}

fn source_base_properties(
    command: &RagPersistIngestCommand,
    node_code: &str,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = BTreeMap::new();
    push_text_prop(&mut properties, "corpusId", &command.corpus_id);
    push_text_prop(&mut properties, "sourceCode", &command.source_code);
    push_text_prop(&mut properties, "sourceVersion", &command.source_version);
    push_text_prop(&mut properties, "sourceStatus", &command.source_status);
    push_text_prop(&mut properties, "scopeType", &command.scope.scope_type);
    push_text_prop(&mut properties, "scopeCode", &command.scope.scope_code);
    push_text_prop(&mut properties, "ragWorkspaceCode", &command.workspace_code);
    push_text_prop(&mut properties, "targetCode", node_code);
    push_text_prop(
        &mut properties,
        "title",
        command.title.as_deref().unwrap_or(""),
    );
    push_text_prop(
        &mut properties,
        "metadataJson",
        &json_string(&command.metadata)?,
    );
    push_text_prop(
        &mut properties,
        "sourceMetadataJson",
        &json_string(&command.metadata)?,
    );
    Ok(properties)
}

pub(super) fn text_unit_properties(
    corpus: &RagCorpusRecord,
    command: &RagPersistIngestCommand,
    unit: &RagTextUnitWrite,
    node_code: &str,
    text_unit_node_code: &str,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = source_base_properties(command, node_code)?;
    push_domain_field_properties(corpus, "textUnit", command, &mut properties)?;
    push_text_prop(&mut properties, "textUnitCode", &unit.text_unit_code);
    push_text_prop(&mut properties, "textUnitNodeCode", text_unit_node_code);
    push_text_prop(&mut properties, "text", &unit.text);
    push_text_prop(&mut properties, "searchText", &unit.search_text);
    push_text_prop(&mut properties, "headingPath", &unit.heading_path);
    push_text_prop(&mut properties, "textUnitStatus", &unit.status);
    push_text_prop(&mut properties, "fulltextStatus", &unit.fulltext_status);
    push_text_prop(&mut properties, "embeddingStatus", &unit.embedding_status);
    push_text_prop(
        &mut properties,
        "embeddingModelCode",
        unit.embedding_model_code.as_deref().unwrap_or(""),
    );
    push_text_prop(
        &mut properties,
        "metadataJson",
        &json_string(&unit.metadata)?,
    );
    properties.insert(
        "ordinal".into(),
        GraphPropertyValue::Integer(unit.ordinal as i64),
    );
    properties.insert(
        "tokenCount".into(),
        GraphPropertyValue::Integer(unit.token_count),
    );
    if let Some(embedding) = unit.embedding.as_deref() {
        push_text_prop(
            &mut properties,
            "embeddingJson",
            &json_string(&decode_embedding(embedding))?,
        );
    }
    Ok(properties)
}

pub(super) fn tree_node_properties(
    _corpus: &RagCorpusRecord,
    command: &RagPersistIngestCommand,
    node: &RagTreeNodeWrite,
    node_code: &str,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = BTreeMap::new();
    push_text_prop(&mut properties, "corpusId", &command.corpus_id);
    push_text_prop(&mut properties, "sourceCode", &node.source_code);
    push_text_prop(&mut properties, "sourceVersion", &node.source_version);
    push_text_prop(&mut properties, "scopeType", &node.scope_type);
    push_text_prop(&mut properties, "scopeCode", &node.scope_code);
    push_text_prop(&mut properties, "sourceStatus", &command.source_status);
    push_text_prop(&mut properties, "ragWorkspaceCode", &command.workspace_code);
    push_text_prop(&mut properties, "targetCode", node_code);
    push_text_prop(&mut properties, "treeNodeCode", &node.tree_node_code);
    push_text_prop(&mut properties, "parentNodeCode", &node.parent_node_code);
    push_text_prop(&mut properties, "pathKey", &node.path_key);
    properties.insert("level".into(), GraphPropertyValue::Integer(node.level));
    properties.insert("ordinal".into(), GraphPropertyValue::Integer(node.ordinal));
    push_text_prop(&mut properties, "title", &node.title);
    push_text_prop(&mut properties, "summaryText", &node.summary_text);
    push_text_prop(&mut properties, "text", &node.summary_text);
    push_text_prop(&mut properties, "searchText", &node.summary_text);
    properties.insert(
        "summaryTokenCount".into(),
        GraphPropertyValue::Integer(node.summary_token_count),
    );
    push_text_prop(
        &mut properties,
        "summaryContentHash",
        &node.summary_content_hash,
    );
    push_text_prop(
        &mut properties,
        "summaryFulltextStatus",
        &node.summary_fulltext_status,
    );
    push_text_prop(
        &mut properties,
        "summaryEmbeddingStatus",
        &node.summary_embedding_status,
    );
    push_text_prop(
        &mut properties,
        "summaryEmbeddingModelCode",
        node.summary_embedding_model_code.as_deref().unwrap_or(""),
    );
    push_text_prop(
        &mut properties,
        "summaryEmbeddingVersion",
        node.summary_embedding_version.as_deref().unwrap_or(""),
    );
    push_text_prop(
        &mut properties,
        "summaryEmbeddingTextHash",
        node.summary_embedding_text_hash.as_deref().unwrap_or(""),
    );
    properties.insert(
        "summaryEmbeddingDimension".into(),
        GraphPropertyValue::Integer(node.summary_embedding_dimension.unwrap_or(0)),
    );
    push_text_prop(
        &mut properties,
        "coveredTextUnitCodes",
        &json_string(&node.covered_text_unit_codes)?,
    );
    properties.insert(
        "coveredTextUnitCount".into(),
        GraphPropertyValue::Integer(node.covered_text_unit_count),
    );
    push_text_prop(
        &mut properties,
        "metadataJson",
        &json_string(&node.metadata)?,
    );
    push_text_prop(&mut properties, "status", &node.status);
    Ok(properties)
}

pub(super) fn tree_edge_properties(
    command: &RagPersistIngestCommand,
    node: &RagTreeNodeWrite,
) -> BTreeMap<String, GraphPropertyValue> {
    let mut properties = BTreeMap::new();
    push_text_prop(&mut properties, "corpusId", &command.corpus_id);
    push_text_prop(&mut properties, "sourceCode", &node.source_code);
    push_text_prop(&mut properties, "sourceVersion", &node.source_version);
    push_text_prop(&mut properties, "scopeType", &node.scope_type);
    push_text_prop(&mut properties, "scopeCode", &node.scope_code);
    properties.insert("ordinal".into(), GraphPropertyValue::Integer(node.ordinal));
    properties
}

pub(super) fn push_domain_field_properties(
    corpus: &RagCorpusRecord,
    segment: &str,
    command: &RagPersistIngestCommand,
    properties: &mut BTreeMap<String, GraphPropertyValue>,
) -> Result<(), RagError> {
    for (logical_name, value) in &command.domain_fields {
        let property_name = graph_property_name_for_logical(corpus, segment, logical_name)
            .ok_or(RagError::StorageBindingInvalid)?;
        if is_reserved_main_node_property(property_name) {
            return Err(RagError::StorageBindingInvalid);
        }
        properties.insert(property_name.to_string(), json_graph_property_value(value)?);
    }
    Ok(())
}

fn is_reserved_main_node_property(name: &str) -> bool {
    matches!(
        name,
        "corpusId"
            | "sourceCode"
            | "sourceVersion"
            | "sourceStatus"
            | "scopeType"
            | "scopeCode"
            | "ragWorkspaceCode"
            | "targetCode"
            | "textUnitCode"
            | "title"
            | "text"
            | "searchText"
            | "headingPath"
            | "textUnitStatus"
            | "fulltextStatus"
            | "embeddingStatus"
            | "embeddingModelCode"
            | "metadataJson"
            | "sourceMetadataJson"
            | "tokenCount"
            | "embeddingJson"
            | "treeNodeCode"
            | "parentNodeCode"
            | "pathKey"
            | "level"
            | "summaryText"
            | "summaryTokenCount"
            | "summaryContentHash"
            | "summaryFulltextStatus"
            | "summaryEmbeddingStatus"
            | "summaryEmbeddingModelCode"
            | "summaryEmbeddingVersion"
            | "summaryEmbeddingTextHash"
            | "summaryEmbeddingDimension"
            | "coveredTextUnitCodes"
            | "coveredTextUnitCount"
    )
}

pub(super) fn custom_related_properties(
    node: &Value,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = BTreeMap::new();
    let Some(raw_properties) = node.get("properties") else {
        return Ok(properties);
    };
    let property_map = raw_properties
        .as_object()
        .ok_or(RagError::IngestRequestInvalid)?;
    for (name, value) in property_map {
        let name = name.trim();
        if name.is_empty() || is_reserved_related_property(name) {
            return Err(RagError::IngestRequestInvalid);
        }
        properties.insert(name.to_string(), json_graph_property_value(value)?);
    }
    Ok(properties)
}

pub(super) fn custom_edge_properties(
    edge: &Value,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = BTreeMap::new();
    let Some(raw_properties) = edge.get("properties") else {
        return Ok(properties);
    };
    let property_map = raw_properties
        .as_object()
        .ok_or(RagError::IngestRequestInvalid)?;
    for (name, value) in property_map {
        let name = name.trim();
        if name.is_empty() || is_reserved_edge_property(name) {
            return Err(RagError::IngestRequestInvalid);
        }
        properties.insert(name.to_string(), json_graph_property_value(value)?);
    }
    Ok(properties)
}

fn is_reserved_related_property(name: &str) -> bool {
    matches!(name, "relatedKind" | "relatedRef" | "title" | "text")
}

fn is_reserved_edge_property(name: &str) -> bool {
    matches!(
        name,
        "edgeCode" | "sourceNodeCode" | "targetNodeCode" | "relationship"
    )
}

fn json_graph_property_value(value: &Value) -> Result<GraphPropertyValue, RagError> {
    match value {
        Value::String(value) => Ok(GraphPropertyValue::Text(value.clone())),
        Value::Number(value) => value
            .as_i64()
            .map(GraphPropertyValue::Integer)
            .or_else(|| value.as_f64().map(GraphPropertyValue::Decimal))
            .ok_or(RagError::IngestRequestInvalid),
        Value::Bool(value) => Ok(GraphPropertyValue::Boolean(*value)),
        _ => Err(RagError::IngestRequestInvalid),
    }
}
