use std::sync::Arc;

use crate::{
    application::graph::{
        GraphEmbeddingRuntime, PreparedGraphEdgeWrite, PreparedGraphNodeWrite,
        PreparedGraphSearchQuery, ScopeResolver, WorkspaceDetailProvider, prepare_edge_write,
        prepare_node_write, prepare_search_query,
    },
    domain::graph::{
        GraphEdge, GraphEdgeWrite, GraphError, GraphNode, GraphNodeWrite, GraphPath,
        GraphScopeCommand, GraphSearchHit, GraphSearchQuery, GraphTraverseQuery,
        ResolvedGraphScope, normalize_graph_code,
    },
};

pub trait GraphScopeResolver: Send + Sync {
    fn resolve_graph_scope(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, GraphError>;
}

impl<W> GraphScopeResolver for ScopeResolver<W>
where
    W: WorkspaceDetailProvider + Send + Sync,
{
    fn resolve_graph_scope(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, GraphError> {
        self.resolve(command).map_err(GraphError::from)
    }
}

pub trait GraphStore: Send + Sync {
    fn create_node(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    ) -> Result<GraphNode, GraphError>;

    fn get_node(
        &self,
        scope: &ResolvedGraphScope,
        node_code: &str,
    ) -> Result<Option<GraphNode>, GraphError>;

    fn update_node(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    ) -> Result<GraphNode, GraphError>;

    fn delete_node(&self, scope: &ResolvedGraphScope, node_code: &str) -> Result<(), GraphError>;

    fn get_edge(
        &self,
        scope: &ResolvedGraphScope,
        edge_code: &str,
    ) -> Result<Option<GraphEdge>, GraphError>;

    fn create_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError>;

    fn update_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError>;

    fn delete_edge(&self, scope: &ResolvedGraphScope, edge_code: &str) -> Result<(), GraphError>;

    fn search(
        &self,
        scope: &ResolvedGraphScope,
        query: PreparedGraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError>;

    fn traverse(
        &self,
        scope: &ResolvedGraphScope,
        query: GraphTraverseQuery,
    ) -> Result<Vec<GraphPath>, GraphError>;
}

#[derive(Clone)]
pub struct GraphService<R = ScopeResolver> {
    store: Arc<dyn GraphStore>,
    scope_resolver: R,
    embedding_runtime: Option<Arc<dyn GraphEmbeddingRuntime>>,
}

impl<R> GraphService<R>
where
    R: GraphScopeResolver,
{
    pub fn new(store: impl GraphStore + 'static, scope_resolver: R) -> Self {
        Self {
            store: Arc::new(store),
            scope_resolver,
            embedding_runtime: None,
        }
    }

    pub fn with_embedding_runtime(
        store: impl GraphStore + 'static,
        scope_resolver: R,
        embedding_runtime: Arc<dyn GraphEmbeddingRuntime>,
    ) -> Self {
        Self {
            store: Arc::new(store),
            scope_resolver,
            embedding_runtime: Some(embedding_runtime),
        }
    }

    pub async fn create_node(
        &self,
        scope: GraphScopeCommand,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let command = command.validate()?;
        let scope = self.resolve_scope(scope)?;
        let command = prepare_node_write(self.embedding_runtime(), &scope, command).await?;
        self.store.create_node(&scope, command)
    }

    pub fn get_node(
        &self,
        scope: GraphScopeCommand,
        node_code: String,
    ) -> Result<GraphNode, GraphError> {
        let node_code = normalize_graph_code(&node_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_node(scope, &node_code)?
                .ok_or(GraphError::MissingNode { node_code })
        })
    }

    pub async fn update_node(
        &self,
        scope: GraphScopeCommand,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let command = command.validate()?;
        let scope = self.resolve_scope(scope)?;
        let command = prepare_node_write(self.embedding_runtime(), &scope, command).await?;
        self.store.update_node(&scope, command)
    }

    pub fn delete_node(
        &self,
        scope: GraphScopeCommand,
        node_code: String,
    ) -> Result<(), GraphError> {
        let node_code = normalize_graph_code(&node_code)?;
        self.with_scope(scope, |store, scope| store.delete_node(scope, &node_code))
    }

    pub fn get_edge(
        &self,
        scope: GraphScopeCommand,
        edge_code: String,
    ) -> Result<GraphEdge, GraphError> {
        let edge_code = normalize_graph_code(&edge_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_edge(scope, &edge_code)?
                .ok_or(GraphError::MissingEdge { edge_code })
        })
    }

    pub async fn create_edge(
        &self,
        scope: GraphScopeCommand,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let command = command.validate()?;
        let scope = self.resolve_scope(scope)?;
        ensure_endpoint_exists(self.store.as_ref(), &scope, &command.source_node_code)?;
        ensure_endpoint_exists(self.store.as_ref(), &scope, &command.target_node_code)?;
        let command = prepare_edge_write(self.embedding_runtime(), &scope, command).await?;
        self.store.create_edge(&scope, command)
    }

    pub async fn update_edge(
        &self,
        scope: GraphScopeCommand,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let command = command.validate()?;
        let scope = self.resolve_scope(scope)?;
        ensure_endpoint_exists(self.store.as_ref(), &scope, &command.source_node_code)?;
        ensure_endpoint_exists(self.store.as_ref(), &scope, &command.target_node_code)?;
        let command = prepare_edge_write(self.embedding_runtime(), &scope, command).await?;
        self.store.update_edge(&scope, command)
    }

    pub fn delete_edge(
        &self,
        scope: GraphScopeCommand,
        edge_code: String,
    ) -> Result<(), GraphError> {
        let edge_code = normalize_graph_code(&edge_code)?;
        self.with_scope(scope, |store, scope| store.delete_edge(scope, &edge_code))
    }

    pub async fn search(
        &self,
        scope: GraphScopeCommand,
        query: GraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError> {
        let query = query.validate()?;
        let scope = self.resolve_scope(scope)?;
        let query = prepare_search_query(self.embedding_runtime(), &scope, query).await?;
        self.store.search(&scope, query)
    }

    pub fn traverse(
        &self,
        scope: GraphScopeCommand,
        query: GraphTraverseQuery,
    ) -> Result<Vec<GraphPath>, GraphError> {
        let query = query.validate()?;
        self.with_scope(scope, |store, scope| store.traverse(scope, query))
    }

    fn with_scope<T>(
        &self,
        scope: GraphScopeCommand,
        operation: impl FnOnce(&dyn GraphStore, &ResolvedGraphScope) -> Result<T, GraphError>,
    ) -> Result<T, GraphError> {
        let scope = self.resolve_scope(scope)?;
        operation(self.store.as_ref(), &scope)
    }

    fn resolve_scope(&self, scope: GraphScopeCommand) -> Result<ResolvedGraphScope, GraphError> {
        self.scope_resolver.resolve_graph_scope(scope)
    }

    fn embedding_runtime(&self) -> Option<&dyn GraphEmbeddingRuntime> {
        self.embedding_runtime
            .as_ref()
            .map(|runtime| runtime.as_ref())
    }
}

fn ensure_endpoint_exists(
    store: &dyn GraphStore,
    scope: &ResolvedGraphScope,
    node_code: &str,
) -> Result<(), GraphError> {
    if store.get_node(scope, node_code)?.is_none() {
        return Err(GraphError::MissingEndpoint {
            node_code: node_code.to_owned(),
        });
    }

    Ok(())
}
