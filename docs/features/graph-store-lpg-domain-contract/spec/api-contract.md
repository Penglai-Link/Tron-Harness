# API Contract: Graph Store LPG Domain Contract

## API Overview

This feature defines internal Rust APIs for LPG graph operations. It adds `GraphService` and `GraphStore` contracts for internal callers, but it adds no HTTP route, controller, frontend API contract, or public query language endpoint.

## Endpoints

- none
- No backend route is added for knowledge graph or memory surfaces.
- No public REST, GraphQL, GQL, Cypher, SPARQL, or Grafeo query endpoint is introduced.

## Internal Rust Contract

| type | layer | role |
| --- | --- | --- |
| `GraphNode` | domain | typed LPG node record |
| `GraphEdge` | domain | typed LPG edge record |
| `GraphPropertyValue` | domain | typed dynamic property value |
| `GraphSearchQuery` | domain | typed hybrid search plan without raw query strings |
| `GraphTraverseQuery` | domain | typed traversal plan |
| `GraphStore` | application port | storage abstraction for nodes, edges, search, and traversal |
| `GraphService` | application service | validates commands, resolves scope, and calls store port |
| `GraphError` | domain/application | typed LPG errors |

Suggested Rust shape:

```rust
pub trait GraphStore: Send {
    fn create_node(&mut self, scope: &ResolvedGraphScope, command: GraphNodeWrite) -> Result<GraphNode, GraphError>;
    fn create_edge(&mut self, scope: &ResolvedGraphScope, command: GraphEdgeWrite) -> Result<GraphEdge, GraphError>;
    fn search(&mut self, scope: &ResolvedGraphScope, query: GraphSearchQuery) -> Result<Vec<GraphSearchHit>, GraphError>;
    fn traverse(&mut self, scope: &ResolvedGraphScope, query: GraphTraverseQuery) -> Result<Vec<GraphPath>, GraphError>;
}
```

## Auth And Permissions

- This feature does not add HTTP authentication or authorization.
- Future callers must continue to enforce their own permissions before calling `GraphService`.
- Graph visibility is enforced through the resolved scope passed to every `GraphStore` call.

## Error Contract

| error | condition |
| --- | --- |
| `InvalidGraphCode` | node, edge, or query identifier is blank or malformed |
| `LabelRequired` | node write has no valid label |
| `RelationshipRequired` | edge write has no valid relationship |
| `ReservedPropertyName` | dynamic property name collides with required graph isolation properties |
| `DuplicateNode` | store reports node already exists |
| `DuplicateEdge` | store reports edge already exists |
| `MissingNode` | node lookup, update, delete, or endpoint check misses a node |
| `MissingEdge` | edge lookup, update, or delete misses an edge |
| `MissingEndpoint` | edge source or target node is missing in the resolved scope |
| `EmptySearchQuery` | search has no structured, full-text, vector text, traversal, or label clause |
| `InvalidTraversalQuery` | traversal lacks start nodes or has invalid depth |
| `RawQueryNotAllowed` | caller attempts to bypass typed query contracts |
| `CallerSuppliedVectorNotAllowed` | caller attempts to pass raw embedding vectors |
| `StorageFailure` | store port reports an unrecoverable storage failure |

## Frontend Backend Boundary

- No frontend code changes.
- No frontend mock routes are converted to real backend routes.
- Browser evidence is not required for this feature.

## Database And Transaction Boundary

- No database transaction is opened by the contract itself.
- `GraphService` must validate before calling the store port.
- Future Grafeo adapter owns storage transaction semantics.
- The service checks edge endpoints by calling the same store port with the same resolved scope.

## Mock Or Adapter Expectations

- Tests use fake `GraphStore` and fake scope resolver implementations.
- The fake store must prove service-to-port calls include the resolved scope and typed commands.
- Concrete Grafeo adapter implementation is deferred to `embedded-grafeo-adapter-foundation`.
