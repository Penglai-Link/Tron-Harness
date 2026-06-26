use serde_json::{Map, Value, json};

use crate::{
    application::{
        graph::{GraphSemanticProjection, PreparedGraphSearchQuery},
        rag::{RagGraphExpansionQuery, RagRouteSearchQuery, RagSearchRow},
    },
    domain::{
        graph::{
            GraphPropertyFilter, GraphPropertyValue, GraphQueryLimit, GraphSearchHit,
            GraphSearchQuery, GraphTraversalClause, GraphTraversalDirection, GraphTraverseQuery,
        },
        rag::{RagCorpusRecord, RagError},
    },
};

use super::{
    GraphBackedRagRepository,
    filters::{append_graph_filters, graph_filter},
    model::{
        graph_hit_to_row, graph_node_json, graph_properties_json, graph_segment_for_retriever,
        graph_target_for_retriever, map_graph_error, optional_graph_target_for_segment, prop_i64,
    },
    tree_rows::{
        covered_text_unit_codes_from_row, graph_text_unit_hit_to_tree_row, graph_tree_hit_to_row,
        tree_output_field_values,
    },
};

impl GraphBackedRagRepository {
    pub(super) fn search_graph_route(
        &self,
        corpus: &RagCorpusRecord,
        query: &RagRouteSearchQuery,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        if matches!(
            query.retriever.as_str(),
            "TREE_FULLTEXT_V1" | "TREE_VECTOR_V1"
        ) {
            return self.search_graph_tree_route(corpus, query);
        }
        let mut rows = Vec::new();
        let segment = graph_segment_for_retriever(&query.retriever)?;
        let target = graph_target_for_retriever(corpus, &query.retriever)?;
        for rag_scope in &query.scopes {
            let scope = self.resolve_graph_scope_for_namespace(
                &target.namespace,
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
            append_graph_filters(&mut filters, corpus, &query.filters, segment)?;
            let full_text = if query.retriever == "FULLTEXT_GRAPH_SINGLE_NODE_V1" {
                Some(query.query.clone())
            } else {
                None
            };
            let graph_query = GraphSearchQuery {
                filters: filters.clone(),
                full_text,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec![target.node_label.clone()],
                limit: GraphQueryLimit::new(query.limit.max(1)).map_err(map_graph_error)?,
                min_score: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            let prepared_query = prepare_graph_search_query(graph_query, query);
            let hits = self
                .graph_store
                .search(&scope, prepared_query)
                .map_err(map_graph_error)?;
            rows.extend(
                hits.into_iter()
                    .map(|hit| graph_hit_to_row(corpus, hit.node, hit.score, segment)),
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

    fn search_graph_tree_route(
        &self,
        corpus: &RagCorpusRecord,
        query: &RagRouteSearchQuery,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let policy = TreeRoutePolicy::from_query(query);
        let target = graph_target_for_retriever(corpus, &query.retriever)?;
        let mut tree_rows = Vec::new();
        for rag_scope in &query.scopes {
            let scope = self.resolve_graph_scope_for_namespace(
                &target.namespace,
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
                graph_filter("status", "ACTIVE"),
            ];
            if !policy.include_root {
                filters.push(GraphPropertyFilter {
                    property_name: "parentNodeCode".into(),
                    operator: crate::domain::graph::GraphFilterOperator::NotEquals,
                    value: GraphPropertyValue::Text("0".into()),
                });
            }
            match query.retriever.as_str() {
                "TREE_FULLTEXT_V1" => {
                    filters.push(graph_filter("summaryFulltextStatus", "INDEXED"));
                }
                "TREE_VECTOR_V1" => {
                    filters.push(graph_filter("summaryEmbeddingStatus", "INDEXED"));
                }
                _ => return Err(RagError::SearchRouteUnavailable),
            }
            append_graph_filters(&mut filters, corpus, &query.filters, "tree")?;
            let full_text = if query.retriever == "TREE_FULLTEXT_V1" {
                Some(query.query.clone())
            } else {
                None
            };
            let graph_query = GraphSearchQuery {
                filters: filters.clone(),
                full_text,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec![target.node_label.clone()],
                limit: GraphQueryLimit::new(query.limit.max(1)).map_err(map_graph_error)?,
                min_score: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            let prepared_query = prepare_graph_search_query(graph_query, query);
            let mut hits = self
                .graph_store
                .search(&scope, prepared_query)
                .map_err(map_graph_error)?;
            sort_tree_hits(&mut hits, &policy);
            tree_rows.extend(hits.into_iter().map(|hit| {
                let mut row = graph_tree_hit_to_row(corpus, hit.node, hit.score, &query.retriever);
                apply_tree_policy_to_row(&mut row, &policy);
                row
            }));
        }
        tree_rows.sort_by(|left, right| {
            right
                .storage_score
                .partial_cmp(&left.storage_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.target_code.cmp(&right.target_code))
        });
        let mut rows = self.hydrate_graph_tree_rows(corpus, query, tree_rows)?;
        rows = dedup_graph_tree_rows(rows);
        rows.truncate(query.limit);
        Ok(rows)
    }

    fn hydrate_graph_tree_rows(
        &self,
        corpus: &RagCorpusRecord,
        query: &RagRouteSearchQuery,
        tree_rows: Vec<RagSearchRow>,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        if optional_graph_target_for_segment(corpus, "textUnit")?.is_some() {
            return self.hydrate_graph_tree_rows_from_graph_text_units(corpus, query, tree_rows);
        }
        if corpus
            .storage_binding
            .sqlite_table_for_segment("textUnit")
            .is_some()
        {
            return self
                .sqlite
                .hydrate_graph_tree_rows_from_text_units(corpus, tree_rows);
        }
        Ok(tree_rows
            .into_iter()
            .map(|mut row| {
                row.field_values = tree_output_field_values(&row);
                row
            })
            .collect())
    }

    fn hydrate_graph_tree_rows_from_graph_text_units(
        &self,
        corpus: &RagCorpusRecord,
        query: &RagRouteSearchQuery,
        tree_rows: Vec<RagSearchRow>,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let target = optional_graph_target_for_segment(corpus, "textUnit")?
            .ok_or(RagError::SearchRouteUnavailable)?;
        let mut rows = Vec::new();
        for tree_row in tree_rows {
            let covered_codes = covered_text_unit_codes_from_row(&tree_row);
            for text_unit_code in covered_codes {
                let scope = self.resolve_graph_scope_for_namespace(
                    &target.namespace,
                    &query.workspace_code,
                    &tree_row.scope_type,
                    &tree_row.scope_code,
                )?;
                let graph_query = GraphSearchQuery {
                    filters: vec![
                        graph_filter("corpusId", &query.corpus_id),
                        graph_filter("sourceCode", &tree_row.source_code),
                        graph_filter("sourceVersion", &tree_row.source_version),
                        graph_filter("scopeType", &tree_row.scope_type),
                        graph_filter("scopeCode", &tree_row.scope_code),
                        graph_filter("sourceStatus", "ACTIVE"),
                        graph_filter("textUnitStatus", "ACTIVE"),
                        graph_filter("textUnitCode", &text_unit_code),
                    ],
                    full_text: None,
                    vector_text: None,
                    embedding_model_code: None,
                    traversal: None,
                    labels: vec![target.node_label.clone()],
                    limit: GraphQueryLimit::new(1).map_err(map_graph_error)?,
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
                        .map(|hit| graph_text_unit_hit_to_tree_row(corpus, hit.node, &tree_row)),
                );
            }
        }
        Ok(dedup_graph_tree_rows(rows))
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
        let relationships = query.relationships.clone();
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
                        "relationship": edge.relationship,
                        "properties": graph_properties_json(&edge.properties)
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

fn dedup_graph_tree_rows(rows: Vec<RagSearchRow>) -> Vec<RagSearchRow> {
    let mut seen = std::collections::BTreeSet::new();
    rows.into_iter()
        .filter(|row| seen.insert(row.target_code.clone()))
        .collect()
}

fn prepare_graph_search_query(
    graph_query: GraphSearchQuery,
    query: &RagRouteSearchQuery,
) -> PreparedGraphSearchQuery {
    let Some(embedding) = query.query_embedding.as_ref() else {
        return PreparedGraphSearchQuery::plain(graph_query);
    };
    PreparedGraphSearchQuery {
        query: graph_query,
        semantic_model_code: query.query_embedding_model_code.clone(),
        semantic_projection: Some(GraphSemanticProjection::from_values(embedding.clone())),
    }
}

#[derive(Debug, Clone, Copy)]
struct TreeRoutePolicy {
    include_root: bool,
    max_covered_units_per_node: Option<usize>,
    prefer_deeper_tree_node: bool,
}

impl TreeRoutePolicy {
    fn from_query(query: &RagRouteSearchQuery) -> Self {
        let policy = query
            .policy
            .get("treePolicy")
            .or_else(|| query.policy.get("policy"))
            .or_else(|| query.policy.get("routePolicy"))
            .unwrap_or(&query.policy);
        Self {
            include_root: policy_bool(policy, "includeRoot", false),
            max_covered_units_per_node: policy_optional_usize(policy, "maxCoveredUnitsPerNode"),
            prefer_deeper_tree_node: policy_bool(policy, "preferDeeperTreeNode", false),
        }
    }
}

fn sort_tree_hits(hits: &mut [GraphSearchHit], policy: &TreeRoutePolicy) {
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                if policy.prefer_deeper_tree_node {
                    prop_i64(&right.node.properties, "level")
                        .unwrap_or_default()
                        .cmp(&prop_i64(&left.node.properties, "level").unwrap_or_default())
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| {
                prop_i64(&left.node.properties, "ordinal")
                    .unwrap_or_default()
                    .cmp(&prop_i64(&right.node.properties, "ordinal").unwrap_or_default())
            })
    });
}

fn apply_tree_policy_to_row(row: &mut RagSearchRow, policy: &TreeRoutePolicy) {
    let Some(max) = policy.max_covered_units_per_node else {
        return;
    };
    let Some(values) = row
        .field_values
        .get("coveredTextUnitCodes")
        .and_then(Value::as_array)
    else {
        return;
    };
    row.field_values.insert(
        "coveredTextUnitCodes".into(),
        Value::Array(values.iter().take(max).cloned().collect()),
    );
}

fn policy_bool(policy: &Value, field: &str, default_value: bool) -> bool {
    policy
        .get(field)
        .and_then(Value::as_bool)
        .unwrap_or(default_value)
}

fn policy_optional_usize(policy: &Value, field: &str) -> Option<usize> {
    policy
        .get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
}

fn policy_usize(policy: &Value, field: &str, default_value: usize) -> usize {
    policy
        .get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        application::rag::{RagRouteSearchQuery, RagSearchScope},
        domain::graph::{GraphQueryLimit, GraphSearchQuery},
    };

    use super::prepare_graph_search_query;

    #[test]
    fn graph_route_query_embedding_becomes_prepared_semantic_projection() {
        let graph_query = GraphSearchQuery {
            labels: vec!["RagSkill".into()],
            limit: GraphQueryLimit::new(5).unwrap(),
            ..GraphSearchQuery::empty()
        };
        let route_query = RagRouteSearchQuery {
            corpus_id: "skill-graph".into(),
            route_id: "skill-graph-vector".into(),
            retriever: "VECTOR_GRAPH_SINGLE_NODE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "semantic skill lookup".into(),
            query_embedding: Some(vec![0.25, 0.5, 0.75]),
            query_embedding_model_code: Some("FAKE_EMBEDDING".into()),
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({}),
            return_fields: Vec::new(),
            limit: 5,
        };

        let prepared = prepare_graph_search_query(graph_query, &route_query);

        assert_eq!(
            prepared.semantic_model_code.as_deref(),
            Some("FAKE_EMBEDDING")
        );
        assert_eq!(
            prepared.semantic_projection.unwrap().as_slice(),
            &[0.25, 0.5, 0.75]
        );
    }
}
