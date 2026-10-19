use crate::application::rag::graph::{
    GraphBrowseInput, GraphExpandInput, GraphReadOptions, GraphSearchInput,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn filters() -> Value {
    json!({})
}
fn seeds() -> usize {
    10
}
fn depth() -> u8 {
    3
}
fn nodes() -> usize {
    100
}
fn edges() -> usize {
    200
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GraphSearchRequest {
    query: String,
    #[serde(default)]
    corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    filters: Value,
    #[serde(default = "seeds")]
    seed_count: usize,
    #[serde(default = "depth")]
    incoming_depth: u8,
    #[serde(default = "depth")]
    outgoing_depth: u8,
    #[serde(default = "nodes")]
    node_limit: usize,
    #[serde(default = "edges")]
    edge_limit: usize,
    #[serde(default)]
    relationship_types: BTreeSet<String>,
    #[serde(default)]
    include_structure: bool,
}
impl GraphSearchRequest {
    pub(super) fn into_input(self) -> GraphSearchInput {
        GraphSearchInput {
            query: self.query,
            corpus_ids: self.corpus_ids,
            filters: self.filters,
            options: GraphReadOptions {
                seed_count: self.seed_count,
                incoming_depth: self.incoming_depth,
                outgoing_depth: self.outgoing_depth,
                node_limit: self.node_limit,
                edge_limit: self.edge_limit,
                relationship_types: self.relationship_types,
                include_structure: self.include_structure,
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GraphExpandRequest {
    #[serde(default)]
    node_ids: Vec<String>,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    filters: Value,
    #[serde(default = "seeds")]
    seed_count: usize,
    #[serde(default = "depth")]
    incoming_depth: u8,
    #[serde(default = "depth")]
    outgoing_depth: u8,
    #[serde(default = "nodes")]
    node_limit: usize,
    #[serde(default = "edges")]
    edge_limit: usize,
    #[serde(default)]
    relationship_types: BTreeSet<String>,
    #[serde(default)]
    include_structure: bool,
}
impl GraphExpandRequest {
    pub(super) fn into_input(self) -> GraphExpandInput {
        GraphExpandInput {
            node_ids: self.node_ids,
            cursor: self.cursor,
            corpus_ids: self.corpus_ids,
            filters: self.filters,
            options: GraphReadOptions {
                seed_count: self.seed_count,
                incoming_depth: self.incoming_depth,
                outgoing_depth: self.outgoing_depth,
                node_limit: self.node_limit,
                edge_limit: self.edge_limit,
                relationship_types: self.relationship_types,
                include_structure: self.include_structure,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GraphBrowseRequest {
    #[serde(default)]
    corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    filters: Value,
    #[serde(default = "seeds")]
    seed_count: usize,
    #[serde(default = "depth")]
    incoming_depth: u8,
    #[serde(default = "depth")]
    outgoing_depth: u8,
    #[serde(default = "nodes")]
    node_limit: usize,
    #[serde(default = "edges")]
    edge_limit: usize,
    #[serde(default)]
    relationship_types: BTreeSet<String>,
    #[serde(default)]
    include_structure: bool,
}
impl GraphBrowseRequest {
    pub(super) fn into_input(self) -> GraphBrowseInput {
        GraphBrowseInput {
            corpus_ids: self.corpus_ids,
            filters: self.filters,
            options: GraphReadOptions {
                seed_count: self.seed_count,
                incoming_depth: self.incoming_depth,
                outgoing_depth: self.outgoing_depth,
                node_limit: self.node_limit,
                edge_limit: self.edge_limit,
                relationship_types: self.relationship_types,
                include_structure: self.include_structure,
            },
        }
    }
}
