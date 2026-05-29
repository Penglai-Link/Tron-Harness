use std::sync::{Arc, Mutex};

use crate::{
    application::graph::{ScopeResolver, WorkspaceDetailProvider},
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

pub trait GraphStore: Send {
    fn create_node(
        &mut self,
        scope: &ResolvedGraphScope,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError>;

    fn get_node(
        &mut self,
        scope: &ResolvedGraphScope,
        node_code: &str,
    ) -> Result<Option<GraphNode>, GraphError>;

    fn update_node(
        &mut self,
        scope: &ResolvedGraphScope,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError>;

    fn delete_node(
        &mut self,
        scope: &ResolvedGraphScope,
        node_code: &str,
    ) -> Result<(), GraphError>;

    fn get_edge(
        &mut self,
        scope: &ResolvedGraphScope,
        edge_code: &str,
    ) -> Result<Option<GraphEdge>, GraphError>;

    fn create_edge(
        &mut self,
        scope: &ResolvedGraphScope,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError>;

    fn update_edge(
        &mut self,
        scope: &ResolvedGraphScope,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError>;

    fn delete_edge(
        &mut self,
        scope: &ResolvedGraphScope,
        edge_code: &str,
    ) -> Result<(), GraphError>;

    fn search(
        &mut self,
        scope: &ResolvedGraphScope,
        query: GraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError>;

    fn traverse(
        &mut self,
        scope: &ResolvedGraphScope,
        query: GraphTraverseQuery,
    ) -> Result<Vec<GraphPath>, GraphError>;
}

#[derive(Clone)]
pub struct GraphService<R = ScopeResolver> {
    store: Arc<Mutex<Box<dyn GraphStore>>>,
    scope_resolver: R,
}

impl<R> GraphService<R>
where
    R: GraphScopeResolver,
{
    pub fn new(store: impl GraphStore + 'static, scope_resolver: R) -> Self {
        Self {
            store: Arc::new(Mutex::new(Box::new(store))),
            scope_resolver,
        }
    }

    pub fn create_node(
        &self,
        scope: GraphScopeCommand,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let command = command.validate()?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .create_node(&scope, command)
    }

    pub fn get_node(
        &self,
        scope: GraphScopeCommand,
        node_code: String,
    ) -> Result<GraphNode, GraphError> {
        let node_code = normalize_graph_code(&node_code)?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .get_node(&scope, &node_code)?
            .ok_or(GraphError::MissingNode { node_code })
    }

    pub fn update_node(
        &self,
        scope: GraphScopeCommand,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let command = command.validate()?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .update_node(&scope, command)
    }

    pub fn delete_node(
        &self,
        scope: GraphScopeCommand,
        node_code: String,
    ) -> Result<(), GraphError> {
        let node_code = normalize_graph_code(&node_code)?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .delete_node(&scope, &node_code)
    }

    pub fn get_edge(
        &self,
        scope: GraphScopeCommand,
        edge_code: String,
    ) -> Result<GraphEdge, GraphError> {
        let edge_code = normalize_graph_code(&edge_code)?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .get_edge(&scope, &edge_code)?
            .ok_or(GraphError::MissingEdge { edge_code })
    }

    pub fn create_edge(
        &self,
        scope: GraphScopeCommand,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let command = command.validate()?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        let mut store = self.store.lock().expect("graph store mutex poisoned");
        ensure_endpoint_exists(store.as_mut(), &scope, &command.source_node_code)?;
        ensure_endpoint_exists(store.as_mut(), &scope, &command.target_node_code)?;
        store.create_edge(&scope, command)
    }

    pub fn update_edge(
        &self,
        scope: GraphScopeCommand,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let command = command.validate()?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        let mut store = self.store.lock().expect("graph store mutex poisoned");
        ensure_endpoint_exists(store.as_mut(), &scope, &command.source_node_code)?;
        ensure_endpoint_exists(store.as_mut(), &scope, &command.target_node_code)?;
        store.update_edge(&scope, command)
    }

    pub fn delete_edge(
        &self,
        scope: GraphScopeCommand,
        edge_code: String,
    ) -> Result<(), GraphError> {
        let edge_code = normalize_graph_code(&edge_code)?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .delete_edge(&scope, &edge_code)
    }

    pub fn search(
        &self,
        scope: GraphScopeCommand,
        query: GraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError> {
        let query = query.validate()?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .search(&scope, query)
    }

    pub fn traverse(
        &self,
        scope: GraphScopeCommand,
        query: GraphTraverseQuery,
    ) -> Result<Vec<GraphPath>, GraphError> {
        let query = query.validate()?;
        let scope = self.scope_resolver.resolve_graph_scope(scope)?;
        self.store
            .lock()
            .expect("graph store mutex poisoned")
            .traverse(&scope, query)
    }
}

fn ensure_endpoint_exists(
    store: &mut dyn GraphStore,
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
