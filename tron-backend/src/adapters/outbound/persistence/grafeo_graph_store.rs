use grafeo::Value;

use super::{
    grafeo_core::GrafeoStoreCore,
    grafeo_graph_records::{
        base_scope_params, build_edge_params, build_node_params, find_exact_edge, find_exact_node,
        load_edges, load_nodes, storage_error,
    },
    grafeo_graph_traversal::{VisibleGraph, filters_match, full_text_match, labels_match},
};
use crate::{
    application::graph::GraphStore,
    domain::graph::{
        GraphEdge, GraphEdgeWrite, GraphError, GraphSearchHit, GraphSearchQuery,
        GraphTraverseQuery, ResolvedGraphScope,
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
        command: crate::domain::graph::GraphNodeWrite,
    ) -> Result<crate::domain::graph::GraphNode, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if find_exact_node(&session, scope, &command.node_code)?.is_some() {
            return Err(GraphError::DuplicateNode {
                node_code: command.node_code,
            });
        }

        let created_at = crate::shared::time::current_timestamp();
        session
            .execute_with_params(
                "INSERT (:TronGraphNode { \
                    nodeCode: $nodeCode, recordLabels: $recordLabels, businessProps: $businessProps, \
                    fullText: $fullText, embeddingText: $embeddingText, \
                    namespace: $namespace, workspaceCode: $workspaceCode, appCode: $appCode, userCode: $userCode, \
                    createdAt: $createdAt, updatedAt: $updatedAt \
                })",
                build_node_params(scope, &command, &created_at, &created_at),
            )
            .map_err(storage_error)?;

        find_exact_node(&session, scope, &command.node_code)?
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
        command: crate::domain::graph::GraphNodeWrite,
    ) -> Result<crate::domain::graph::GraphNode, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if find_exact_node(&session, scope, &command.node_code)?.is_none() {
            return Err(GraphError::MissingNode {
                node_code: command.node_code,
            });
        }

        let updated_at = crate::shared::time::current_timestamp();
        session
            .execute_with_params(
                "MATCH (n:TronGraphNode) \
                 WHERE n.nodeCode = $nodeCode AND n.namespace = $namespace AND n.workspaceCode = $workspaceCode \
                   AND n.appCode = $appCode AND n.userCode = $userCode \
                 SET n.recordLabels = $recordLabels, n.businessProps = $businessProps, \
                     n.fullText = $fullText, n.embeddingText = $embeddingText, n.updatedAt = $updatedAt",
                build_node_params(scope, &command, &updated_at, &updated_at),
            )
            .map_err(storage_error)?;

        find_exact_node(&session, scope, &command.node_code)?
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
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if find_exact_edge(&session, scope, &command.edge_code)?.is_some() {
            return Err(GraphError::DuplicateEdge {
                edge_code: command.edge_code,
            });
        }

        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        let source = graph
            .preferred_node(&command.source_node_code)
            .ok_or_else(|| GraphError::MissingEndpoint {
                node_code: command.source_node_code.clone(),
            })?;
        let target = graph
            .preferred_node(&command.target_node_code)
            .ok_or_else(|| GraphError::MissingEndpoint {
                node_code: command.target_node_code.clone(),
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
                    embeddingText: $embeddingText, namespace: $namespace, workspaceCode: $workspaceCode, \
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

        find_exact_edge(&session, scope, &command.edge_code)?
            .map(|record| record.edge)
            .ok_or(GraphError::StorageFailure)
    }

    fn update_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if find_exact_edge(&session, scope, &command.edge_code)?.is_none() {
            return Err(GraphError::MissingEdge {
                edge_code: command.edge_code,
            });
        }

        let updated_at = crate::shared::time::current_timestamp();
        let mut params = build_edge_params(
            scope,
            &command,
            scope.storage_dimensions.workspace_code.as_str(),
            scope.storage_dimensions.workspace_code.as_str(),
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
                     e.fullText = $fullText, e.embeddingText = $embeddingText, e.updatedAt = $updatedAt",
                params,
            )
            .map_err(storage_error)?;

        find_exact_edge(&session, scope, &command.edge_code)?
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
        query: GraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError> {
        if query.vector_text.is_some() {
            return Err(GraphError::UnsupportedStoreBehavior);
        }

        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let graph = VisibleGraph::from_scope(scope, load_nodes(&session)?, load_edges(&session)?);
        let candidate_codes = if let Some(traversal) = &query.traversal {
            graph.search_traversal(traversal)
        } else {
            graph.all_preferred_codes()
        };

        let mut hits = Vec::new();
        for code in candidate_codes {
            let Some(record) = graph.preferred_node(&code) else {
                continue;
            };
            if !labels_match(&record.node.labels, &query.labels)
                || !filters_match(&record.node.properties, &query.filters)
                || !full_text_match(record.node.full_text.as_deref(), query.full_text.as_deref())
            {
                continue;
            }

            let score = query
                .full_text
                .as_ref()
                .map(|_| 1.0)
                .or_else(|| query.min_score.map(|_| 1.0));
            if let Some(min_score) = query.min_score
                && score.unwrap_or(1.0) < min_score
            {
                continue;
            }

            hits.push(GraphSearchHit {
                node: record.node.clone(),
                score,
            });
            if hits.len() >= query.limit.value() {
                break;
            }
        }
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
