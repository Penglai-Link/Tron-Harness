# API Contract: Embedded Grafeo Adapter Foundation

## API Overview

This feature adds no HTTP endpoint and no public query language surface. It introduces only internal Rust adapter and runtime contracts so existing `GraphService` and `OntologyService` can use embedded Grafeo through the already-defined typed ports.

## Endpoints

- none
- No backend route is added for `/api/v1/knowledge-graph`.
- No backend route is added for `/api/v1/memory/fragments`.
- No public REST, GraphQL, GQL, Cypher, SPARQL, Turtle, or SHACL endpoint is introduced.

## Internal Rust Contract

| type | layer | role |
| --- | --- | --- |
| `GraphStore` | application port | typed LPG persistence contract |
| `OntologyStore` | application port | typed ontology persistence contract |
| `GrafeoGraphStore` | outbound adapter | embedded Grafeo LPG implementation |
| `GrafeoOntologyStore` | outbound adapter | embedded Grafeo ontology implementation |
| `GrafeoAdapterConfig` | outbound adapter | embedded catalog configuration |
| `GraphService` | application service | validates, resolves scope, and calls `GraphStore` |
| `OntologyService` | application service | validates, resolves scope, and calls `OntologyStore` |

Suggested Rust shape:

```rust
pub struct GrafeoAdapterConfig {
    pub catalog_path: String,
}

pub struct GrafeoGraphStore {
    // owns or shares the embedded Grafeo database handle
}

impl GraphStore for GrafeoGraphStore { /* ... */ }

pub struct GrafeoOntologyStore {
    // owns or shares the embedded Grafeo database handle
}

impl OntologyStore for GrafeoOntologyStore { /* ... */ }
```

## Auth And Permissions

- This feature adds no new HTTP authentication or authorization boundary.
- Future internal callers must continue to authorize before calling graph services.
- The adapter enforces storage isolation only through `ResolvedGraphScope`, not through user-session policy decisions.

## Error Contract

| error | condition |
| --- | --- |
| `StorageFailure` | embedded Grafeo operation fails and the adapter cannot recover |
| `UnsupportedStoreBehavior` | Grafeo capability does not satisfy required LPG, ontology, named-graph, or scope semantics |
| typed duplicate or missing graph or ontology errors | Grafeo indicates the target record is missing or already exists |
| existing validation errors | request never reaches the adapter because service validation or scope resolution failed first |

## Frontend Backend Boundary

- No frontend code changes are required to consume this feature.
- No frontend mock route is converted to a backend route in this slice.
- Browser evidence is not required because this feature remains backend-internal.

## Database And Transaction Boundary

- Grafeo session and transaction behavior are owned by the outbound adapter.
- Service-layer validation and scope resolution happen before any Grafeo call.
- Mutations should be atomic at the adapter operation boundary where Grafeo provides transaction guarantees.
- The adapter must not relax lineage visibility or named-graph isolation to work around capability gaps.

## Mock Or Adapter Expectations

- Port-level unit tests may continue to use fake stores for service behavior.
- This feature adds adapter contract tests backed by temporary embedded Grafeo storage.
- Internal adapter tests may use Grafeo-native setup helpers, but public graph-domain tests continue to assert only typed graph and ontology contracts.
