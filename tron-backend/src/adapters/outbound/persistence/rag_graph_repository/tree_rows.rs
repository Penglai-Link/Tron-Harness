use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{
    application::rag::RagSearchRow,
    domain::{
        graph::{GraphNode, GraphPropertyValue},
        rag::RagCorpusRecord,
    },
};

use super::model::{graph_hit_to_row, prop_i64, prop_text};

pub(super) fn graph_tree_hit_to_row(
    corpus: &RagCorpusRecord,
    node: GraphNode,
    score: Option<f64>,
    retriever: &str,
) -> RagSearchRow {
    let props = &node.properties;
    let metadata = prop_text(props, "metadataJson")
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_else(|| json!({}));
    RagSearchRow {
        corpus_id: corpus.corpus_id.clone(),
        source_code: prop_text(props, "sourceCode")
            .unwrap_or_default()
            .to_string(),
        source_version: prop_text(props, "sourceVersion")
            .unwrap_or_default()
            .to_string(),
        target_code: prop_text(props, "treeNodeCode")
            .unwrap_or(node.node_code.as_str())
            .to_string(),
        scope_type: prop_text(props, "scopeType")
            .unwrap_or_default()
            .to_string(),
        scope_code: prop_text(props, "scopeCode")
            .unwrap_or_default()
            .to_string(),
        title: prop_text(props, "title").map(ToOwned::to_owned),
        text: prop_text(props, "summaryText")
            .or(node.full_text.as_deref())
            .unwrap_or_default()
            .to_string(),
        search_text: prop_text(props, "searchText")
            .or_else(|| prop_text(props, "summaryText"))
            .unwrap_or_default()
            .to_string(),
        heading_path: Vec::new(),
        ordinal: prop_i64(props, "ordinal").unwrap_or(0),
        token_count: prop_i64(props, "summaryTokenCount").unwrap_or(1),
        embedding: None,
        storage_score: score,
        source_metadata: metadata,
        text_metadata: json!({}),
        field_values: graph_tree_field_values(props, score, retriever),
    }
}

pub(super) fn graph_text_unit_hit_to_tree_row(
    corpus: &RagCorpusRecord,
    node: GraphNode,
    tree_row: &RagSearchRow,
) -> RagSearchRow {
    let text_unit_code = prop_text(&node.properties, "textUnitCode")
        .unwrap_or(node.node_code.as_str())
        .to_string();
    let mut row = graph_hit_to_row(corpus, node, tree_row.storage_score, "textUnit");
    row.target_code = text_unit_code;
    row.storage_score = tree_row.storage_score;
    row.field_values.extend(tree_output_field_values(tree_row));
    row
}

pub(super) fn covered_text_unit_codes_from_row(row: &RagSearchRow) -> Vec<String> {
    row.field_values
        .get("coveredTextUnitCodes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn tree_output_field_values(row: &RagSearchRow) -> Map<String, Value> {
    let mut values = row.field_values.clone();
    values.remove("coveredTextUnitCodes");
    values
}

fn graph_tree_field_values(
    properties: &BTreeMap<String, GraphPropertyValue>,
    score: Option<f64>,
    retriever: &str,
) -> Map<String, Value> {
    let mut values = Map::new();
    values.insert("matchedBy".into(), json!(retriever));
    values.insert(
        "treeNodeCode".into(),
        json!(prop_text(properties, "treeNodeCode").unwrap_or_default()),
    );
    values.insert(
        "treeNodeTitle".into(),
        json!(prop_text(properties, "title").unwrap_or_default()),
    );
    if let Some(score) = score {
        values.insert("treeScore".into(), json!(score));
    }
    values.insert(
        "coveredTextUnitCodes".into(),
        prop_text(properties, "coveredTextUnitCodes")
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .unwrap_or_else(|| json!([])),
    );
    values
}
