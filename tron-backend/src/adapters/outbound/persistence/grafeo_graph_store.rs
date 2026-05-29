use std::cmp::Ordering;

use grafeo::Value;

use super::{
    grafeo_core::GrafeoStoreCore,
    grafeo_graph_records::{
        StoredGraphNodeRecord, base_scope_params, build_edge_params, build_node_params,
        find_exact_edge, find_exact_node, load_edges, load_nodes, storage_error,
    },
    grafeo_graph_traversal::{VisibleGraph, filters_match, full_text_match, labels_match},
};
use crate::{
    application::graph::{
        GraphStore, PreparedGraphEdgeWrite, PreparedGraphNodeWrite, PreparedGraphSearchQuery,
    },
    domain::graph::{
        GraphEdge, GraphError, GraphSearchHit, GraphTraverseQuery, ResolvedGraphScope,
    },
};

#[derive(Clone)]
pub struct GrafeoGraphStore {
    core: GrafeoStoreCore,
}

impl GrafeoGraphStore {
    pub fn new(core: GrafeoStoreCore) -> Self {
        Self { core }
    }
}

impl GraphStore for GrafeoGraphStore {
    fn create_node(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    ) -> Result<crate::domain::graph::GraphNode, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let node_code = command.command.node_code.clone();
        if find_exact_node(&session, scope, &node_code)?.is_some() {
            return Err(GraphError::DuplicateNode { node_code });
        }

        let created_at = crate::shared::time::current_timestamp();
        session
            .execute_with_params(
                "INSERT (:TronGraphNode { \
                    nodeCode: $nodeCode, recordLabels: $recordLabels, businessProps: $businessProps, \
                    fullText: $fullText, embeddingText: $embeddingText, semanticPayload: $semanticPayload, \
                    namespace: $namespace, workspaceCode: $workspaceCode, appCode: $appCode, userCode: $userCode, \
                    createdAt: $createdAt, updatedAt: $updatedAt \
                })",
                build_node_params(scope, &command, &created_at, &created_at),
            )
            .map_err(storage_error)?;

        find_exact_node(&session, scope, &command.command.node_code)?
            .map(|record| record.node)
            .ok_or(GraphError::StorageFailure)
    }

    fn get_node(
        &self,
        scope: &ResolvedGraphScope,
        node_code: &str,
    ) -> Result<Option<crate::domain::graph::GraphNode>, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        Ok(graph
            .preferred_node(node_code)
            .map(|record| record.node.clone()))
    }

    fn update_node(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    ) -> Result<crate::domain::graph::GraphNode, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let node_code = command.command.node_code.clone();
        if find_exact_node(&session, scope, &node_code)?.is_none() {
            return Err(GraphError::MissingNode { node_code });
        }

        let updated_at = crate::shared::time::current_timestamp();
        session
            .execute_with_params(
                "MATCH (n:TronGraphNode) \
                 WHERE n.nodeCode = $nodeCode AND n.namespace = $namespace AND n.workspaceCode = $workspaceCode \
                   AND n.appCode = $appCode AND n.userCode = $userCode \
                 SET n.recordLabels = $recordLabels, n.businessProps = $businessProps, \
                     n.fullText = $fullText, n.embeddingText = $embeddingText, \
                     n.semanticPayload = $semanticPayload, n.updatedAt = $updatedAt",
                build_node_params(scope, &command, &updated_at, &updated_at),
            )
            .map_err(storage_error)?;

        find_exact_node(&session, scope, &command.command.node_code)?
            .map(|record| record.node)
            .ok_or(GraphError::StorageFailure)
    }

    fn delete_node(&self, scope: &ResolvedGraphScope, node_code: &str) -> Result<(), GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if find_exact_node(&session, scope, node_code)?.is_none() {
            return Err(GraphError::MissingNode {
                node_code: node_code.to_owned(),
            });
        }

        let mut params = base_scope_params(scope);
        params.insert("nodeCode".into(), Value::from(node_code.to_owned()));
        session
            .execute_with_params(
                "MATCH (n:TronGraphNode) \
                 WHERE n.nodeCode = $nodeCode AND n.namespace = $namespace AND n.workspaceCode = $workspaceCode \
                   AND n.appCode = $appCode AND n.userCode = $userCode \
                 DETACH DELETE n",
                params,
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn get_edge(
        &self,
        scope: &ResolvedGraphScope,
        edge_code: &str,
    ) -> Result<Option<GraphEdge>, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        Ok(graph
            .preferred_edge(edge_code)
            .map(|record| record.edge.clone()))
    }

    fn create_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let edge_code = command.command.edge_code.clone();
        if find_exact_edge(&session, scope, &edge_code)?.is_some() {
            return Err(GraphError::DuplicateEdge { edge_code });
        }

        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        let source = graph
            .preferred_node(&command.command.source_node_code)
            .ok_or_else(|| GraphError::MissingEndpoint {
                node_code: command.command.source_node_code.clone(),
            })?;
        let target = graph
            .preferred_node(&command.command.target_node_code)
            .ok_or_else(|| GraphError::MissingEndpoint {
                node_code: command.command.target_node_code.clone(),
            })?;

        let created_at = crate::shared::time::current_timestamp();
        session
            .execute_with_params(
                "MATCH (a:TronGraphNode), (b:TronGraphNode) \
                 WHERE a.nodeCode = $sourceNodeCode AND a.namespace = $namespace AND a.workspaceCode = $sourceWorkspaceCode \
                   AND a.appCode = $appCode AND a.userCode = $userCode \
                   AND b.nodeCode = $targetNodeCode AND b.namespace = $namespace AND b.workspaceCode = $targetWorkspaceCode \
                   AND b.appCode = $appCode AND b.userCode = $userCode \
                 INSERT (a)-[:TRON_EDGE { \
                    edgeCode: $edgeCode, sourceNodeCode: $sourceNodeCode, targetNodeCode: $targetNodeCode, \
                    relationship: $relationship, businessProps: $businessProps, fullText: $fullText, \
                    embeddingText: $embeddingText, semanticPayload: $semanticPayload, \
                    namespace: $namespace, workspaceCode: $workspaceCode, \
                    appCode: $appCode, userCode: $userCode, createdAt: $createdAt, updatedAt: $updatedAt \
                 }]->(b)",
                build_edge_params(
                    scope,
                    &command,
                    &source.workspace_code,
                    &target.workspace_code,
                    &created_at,
                    &created_at,
                ),
            )
            .map_err(storage_error)?;

        find_exact_edge(&session, scope, &command.command.edge_code)?
            .map(|record| record.edge)
            .ok_or(GraphError::StorageFailure)
    }

    fn update_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let edge_code = command.command.edge_code.clone();
        if find_exact_edge(&session, scope, &edge_code)?.is_none() {
            return Err(GraphError::MissingEdge { edge_code });
        }

        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        let source = graph
            .preferred_node(&command.command.source_node_code)
            .ok_or_else(|| GraphError::MissingEndpoint {
                node_code: command.command.source_node_code.clone(),
            })?;
        let target = graph
            .preferred_node(&command.command.target_node_code)
            .ok_or_else(|| GraphError::MissingEndpoint {
                node_code: command.command.target_node_code.clone(),
            })?;

        let updated_at = crate::shared::time::current_timestamp();
        let mut params = build_edge_params(
            scope,
            &command,
            &source.workspace_code,
            &target.workspace_code,
            &updated_at,
            &updated_at,
        );
        params.insert("updatedAt".into(), Value::from(updated_at));
        session
            .execute_with_params(
                "MATCH ()-[e:TRON_EDGE]->() \
                 WHERE e.edgeCode = $edgeCode AND e.namespace = $namespace AND e.workspaceCode = $workspaceCode \
                   AND e.appCode = $appCode AND e.userCode = $userCode \
                 SET e.sourceNodeCode = $sourceNodeCode, e.targetNodeCode = $targetNodeCode, \
                     e.relationship = $relationship, e.businessProps = $businessProps, \
                     e.fullText = $fullText, e.embeddingText = $embeddingText, \
                     e.semanticPayload = $semanticPayload, e.updatedAt = $updatedAt",
                params,
            )
            .map_err(storage_error)?;

        find_exact_edge(&session, scope, &command.command.edge_code)?
            .map(|record| record.edge)
            .ok_or(GraphError::StorageFailure)
    }

    fn delete_edge(&self, scope: &ResolvedGraphScope, edge_code: &str) -> Result<(), GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if find_exact_edge(&session, scope, edge_code)?.is_none() {
            return Err(GraphError::MissingEdge {
                edge_code: edge_code.to_owned(),
            });
        }

        let mut params = base_scope_params(scope);
        params.insert("edgeCode".into(), Value::from(edge_code.to_owned()));
        session
            .execute_with_params(
                "MATCH ()-[e:TRON_EDGE]->() \
                 WHERE e.edgeCode = $edgeCode AND e.namespace = $namespace AND e.workspaceCode = $workspaceCode \
                   AND e.appCode = $appCode AND e.userCode = $userCode \
                 DELETE e",
                params,
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn search(
        &self,
        scope: &ResolvedGraphScope,
        query: PreparedGraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        let candidate_codes = if let Some(traversal) = &query.query.traversal {
            graph.search_traversal(traversal)
        } else {
            graph.all_preferred_codes()
        };

        let mut hits = Vec::new();
        for code in candidate_codes {
            let Some(record) = graph.preferred_node(&code) else {
                continue;
            };
            if !labels_match(&record.node.labels, &query.query.labels)
                || !filters_match(&record.node.properties, &query.query.filters)
                || !full_text_match(
                    record.node.full_text.as_deref(),
                    query.query.full_text.as_deref(),
                )
            {
                continue;
            }

            let vector_score = query.semantic_projection.as_ref().and_then(|projection| {
                semantic_similarity(
                    record,
                    query.semantic_model_code.as_deref(),
                    projection.as_slice(),
                )
            });
            if query.semantic_projection.is_some() && vector_score.is_none() {
                continue;
            }

            let score = fused_score(
                vector_score,
                query.query.full_text.is_some(),
                query.query.traversal.is_some(),
            );
            if let Some(min_score) = query.query.min_score
                && score.unwrap_or(1.0) < min_score
            {
                continue;
            }

            hits.push(GraphSearchHit {
                node: record.node.clone(),
                score,
            });
        }

        hits.sort_by(
            |left, right| match compare_optional_score(right.score, left.score) {
                Ordering::Equal => left.node.node_code.cmp(&right.node.node_code),
                other => other,
            },
        );
        hits.truncate(query.query.limit.value());
        Ok(hits)
    }

    fn traverse(
        &self,
        scope: &ResolvedGraphScope,
        query: GraphTraverseQuery,
    ) -> Result<Vec<crate::domain::graph::GraphPath>, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        Ok(graph.traverse(&query))
    }
}

fn semantic_similarity(
    record: &StoredGraphNodeRecord,
    model_code: Option<&str>,
    query_projection: &[f32],
) -> Option<f64> {
    if record.semantic_payload.model_code.as_deref() != model_code {
        return None;
    }

    let base_score = record
        .semantic_payload
        .projection
        .as_deref()
        .and_then(|projection| cosine_similarity(projection, query_projection));
    let chunk_score = record
        .semantic_payload
        .slices
        .iter()
        .filter_map(|slice| cosine_similarity(&slice.projection, query_projection))
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));

    match (base_score, chunk_score) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(score), None) | (None, Some(score)) => Some(score),
        (None, None) => None,
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f64> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }

    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (left, right) in left.iter().zip(right.iter()) {
        let left = f64::from(*left);
        let right = f64::from(*right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return None;
    }

    Some(dot / (left_norm.sqrt() * right_norm.sqrt()))
}

fn fused_score(vector_score: Option<f64>, has_full_text: bool, has_traversal: bool) -> Option<f64> {
    let mut components = Vec::new();
    if let Some(vector_score) = vector_score {
        components.push(vector_score);
    }
    if has_full_text {
        components.push(1.0);
    }
    if has_traversal {
        components.push(1.0);
    }

    (!components.is_empty()).then(|| components.iter().sum::<f64>() / components.len() as f64)
}

fn compare_optional_score(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}
