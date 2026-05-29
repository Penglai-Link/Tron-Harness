# API Contract: Graph Scope And Isolation Kernel

## API Overview

This feature defines internal Rust APIs only. It exposes graph scope command and resolved storage-scope types for future graph services and adapters, but it adds no HTTP endpoint, route, controller, frontend contract, or public query language surface.

## Endpoints

- none
- No `GET`, `POST`, `PUT`, `PATCH`, or `DELETE` route is added.
- Existing frontend endpoint strings for knowledge graph or memory remain untouched.

## Internal Rust Contract

| type | layer | role |
| --- | --- | --- |
| `GraphScopeCommand` | domain | raw caller scope input |
| `GraphScope` | domain | validated caller scope with optional app and user dimensions |
| `ResolvedGraphScope` | domain | storage-ready scope with schema, named graph, lineage, and dimensions |
| `GraphStorageDimensions` | domain | concrete required isolation property values |
| `GraphStorageProperty` | domain | reserved future storage property names |
| `GraphScopeError` | domain | typed invalid-scope and resolution error taxonomy |
| `ScopeResolver` | application | resolves `GraphScopeCommand` with `WorkspaceService` |

Suggested Rust shape:

```rust
pub struct GraphScopeCommand {
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: Option<String>,
    pub user_code: Option<String>,
}

impl ScopeResolver {
    pub fn resolve(&self, command: GraphScopeCommand) -> Result<ResolvedGraphScope, GraphScopeError>;
}
```

## Auth And Permissions

- This feature does not implement HTTP authentication or role authorization.
- Scope visibility is data-boundary preparation, not a user permission check.
- Future callers remain responsible for their existing authorization before invoking graph services.
- The resolver rejects invalid or archived workspace scope before any future store adapter call.

## Error Contract

| error | condition |
| --- | --- |
| `InvalidNamespace` | namespace is blank, malformed, or not schema-safe |
| `InvalidWorkspaceCode` | workspace code is blank before workspace lookup |
| `ArchivedWorkspace` | current workspace detail is not active |
| `EmptyWorkspaceLineage` | workspace detail has no path item |
| `WorkspaceResolutionFailed` | `WorkspaceService::get_workspace` fails |
| `ReservedSentinelValue` | app or user code explicitly equals `__NONE__` |
| `ReservedPropertyName` | future dynamic property attempts to use a required storage property name |

## Frontend Backend Boundary

- No frontend change is part of this feature.
- No backend route is added for frontend graph or memory pages.
- Current frontend knowledge graph and memory endpoint strings stay future-facing and are not implemented here.

## Database And Transaction Boundary

- The resolver performs no write transaction.
- The resolver does not open SQL or Grafeo connections directly.
- The only persistence-adjacent dependency is the existing `WorkspaceService::get_workspace` call.
- Later graph and ontology stores must call the resolver before opening adapter-specific write behavior.

## Mock Or Adapter Expectations

- No mock HTTP adapter is needed.
- No Grafeo adapter is needed.
- Unit tests can use a small test resolver dependency or in-memory workspace detail fixtures to simulate workspace outcomes.
- Future Grafeo adapter tests must assert concrete catalog/schema/named-graph behavior against the fields produced here.
