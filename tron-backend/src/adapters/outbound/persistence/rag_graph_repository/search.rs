use serde_json::{Map, Value, json};

use crate::{
    application::{
        graph::PreparedGraphSearchQuery,
        rag::{RagGraphExpansionQuery, RagRouteSearchQuery, RagSearchRow},
    },
    domain::{
        graph::{
            GraphQueryLimit, GraphSearchQuery, GraphTraversalClause, GraphTraversalDirection,
            GraphTraverseQuery,
        },
        rag::{RagCorpusRecord, RagError},
    },
};

use super::{
    GraphBackedRagRepository,
    model::{
        append_graph_filters, graph_filter, graph_hit_to_row, graph_node_json, map_graph_error,
        node_label,
    },
};

impl GraphBackedRagRepository {
    pub(super) fn search_graph_route(
        &self,
        corpus: &RagCorpusRecord,
        query: &RagRouteSearchQuery,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let mut rows = Vec::new();
        for rag_scope in &query.scopes {
            let scope = self.resolve_graph_scope(
                corpus,
                &query.workspace_code,
                &rag_scope.scope_type,
                &rag_scope.scope_code,
            )?;
            let mut filters = vec![
                graph_filter("corpusId", &query.corpus_id),
                graph_filter("sourceVersion", &query.source_version),
                graph_filter("scopeType", &rag_scope.scope_type),
                graph_filter("scopeCode", &rag_scope.scope_code),
                graph_filter("sourceStatus", "ACTIVE"),
                graph_filter("textUnitStatus", "ACTIVE"),
            ];
            append_graph_filters(&mut filters, &query.filters);
            let full_text = if query.retriever == "FULLTEXT_GRAPH_SINGLE_NODE_V1" {
                Some(query.query.clone())
            } else {
                None
            };
            let graph_query = GraphSearchQuery {
                filters,
                full_text,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec![node_label(corpus)?],
                limit: GraphQueryLimit::new(query.limit.max(1)).map_err(map_graph_error)?,
                min_score: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            let hits = self
                .graph_store
                .search(&scope, PreparedGraphSearchQuery::plain(graph_query))
                .map_err(map_graph_error)?;
            rows.extend(
                hits.into_iter()
                    .map(|hit| graph_hit_to_row(corpus, hit.node, hit.score)),
            );
        }
        rows.sort_by(|left, right| {
            right
                .storage_score
                .partial_cmp(&left.storage_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.source_code.cmp(&right.source_code))
        });
        rows.truncate(query.limit);
        Ok(rows)
    }

    pub(super) fn expand_graph_hit(
        &self,
        corpus: &RagCorpusRecord,
        query: &RagGraphExpansionQuery,
    ) -> Result<Option<Value>, RagError> {
        if query.target_code.trim().is_empty() {
            return Ok(None);
        }
        let scope = self.resolve_graph_scope(
            corpus,
            &query.workspace_code,
            &query.scope_type,
            &query.scope_code,
        )?;
        let relationships = query
            .policy
            .get("relationships")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if relationships.is_empty() {
            return Ok(None);
        }
        let max_depth = policy_usize(&query.policy, "maxDepth", 1);
        let max_nodes = policy_usize(&query.policy, "maxRelatedNodes", 20);
        let direction = match query
            .policy
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("OUTGOING")
        {
            "INCOMING" => GraphTraversalDirection::Incoming,
            "ANY" => GraphTraversalDirection::Any,
            _ => GraphTraversalDirection::Outgoing,
        };
        let traverse = GraphTraverseQuery {
            start_node_codes: vec![query.target_code.clone()],
            clauses: vec![GraphTraversalClause {
                direction,
                relationships,
                min_depth: 1,
                max_depth,
            }],
            filters: Vec::new(),
            limit: GraphQueryLimit::new(max_nodes).map_err(map_graph_error)?,
        }
        .validate()
        .map_err(map_graph_error)?;
        let paths = self
            .graph_store
            .traverse(&scope, traverse)
            .map_err(map_graph_error)?;
        let mut nodes = Map::new();
        let mut edges = Map::new();
        for path in paths {
            for node in path
                .nodes
                .into_iter()
                .filter(|node| node.node_code != query.target_code)
            {
                nodes
                    .entry(node.node_code.clone())
                    .or_insert_with(|| graph_node_json(&node));
                if nodes.len() >= max_nodes {
                    break;
                }
            }
            for edge in path.edges {
                edges.entry(edge.edge_code.clone()).or_insert_with(|| {
                    json!({
                        "edgeCode": edge.edge_code,
                        "sourceNodeCode": edge.source_node_code,
                        "targetNodeCode": edge.target_node_code,
                        "relationship": edge.relationship
                    })
                });
            }
        }
        Ok(Some(json!({
            "nodes": nodes.into_values().collect::<Vec<_>>(),
            "edges": edges.into_values().collect::<Vec<_>>()
        })))
    }
}

fn policy_usize(policy: &Value, field: &str, default_value: usize) -> usize {
    policy
        .get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}
