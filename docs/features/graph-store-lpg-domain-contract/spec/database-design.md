# Database Design: Graph Store LPG Domain Contract

## Database Decision

- status: not_applicable
- mandatory_database_change: no
- rationale: this feature defines LPG domain and application contracts only; concrete Grafeo persistence and indexes belong to the embedded adapter feature.

## Existing Schema Impact

- Existing SQLite tables are unchanged.
- Existing workspace schema remains the source for scope resolution through the completed scope kernel.
- No Grafeo catalog, schema, named graph, node collection, edge collection, or index is created in this feature.

## Proposed Schema

### Phase 1: Typed Port Contract

- no SQL table
- no SQL migration
- no Grafeo dependency
- no Grafeo schema object

### Future Grafeo Adapter Phase

The later adapter feature will map these contract values to Grafeo:

| contract value | future storage use |
| --- | --- |
| `GraphNode.node_code` | stable node identifier |
| `GraphNode.labels` | LPG labels |
| `GraphEdge.edge_code` | stable edge identifier |
| `GraphEdge.relationship` | edge relationship type |
| `GraphProperties` | typed dynamic node and edge properties |
| `ResolvedGraphScope` | schema, named graph, lineage filters, and required properties |
| `GraphSearchQuery` | structured, full-text, vector text, and traversal plan |

## Relationships And Cardinality

- One resolved scope contains many nodes.
- One resolved scope contains many edges.
- One edge belongs to exactly one resolved scope and references two nodes in the same resolved scope.
- One node can have many labels and many dynamic properties.
- One edge can have many dynamic properties.
- A search query can combine zero or more labels, zero or more structured filters, optional full-text, optional vector text, and optional traversal clause, but it must include at least one clause.

## Migration Plan

- No migration is required.
- No fixture backfill is required.
- Future adapter work must add its own Grafeo capability verification and storage contract tests.

## Query And Performance Plan

- This feature defines query shape, not query execution.
- Search queries carry limits so future adapters can bound result sizes.
- Traversal queries carry max depth and limit so future adapters can bound traversals.
- Structured filters, labels, full-text, vector text, and traversal clauses stay typed so future adapters can choose efficient indexes.

## Data Governance

- Required graph isolation property names are reserved and cannot be used as dynamic business properties.
- Caller-supplied vectors are not accepted as normal API input.
- Raw query strings are not accepted from callers.
- Full-text and embedding text are caller-provided text fields that later features must treat as sensitive when logging or embedding.

## Test Data And Fixtures

- Unit tests use fake scope resolver and fake graph store.
- Fixtures should include duplicate node, missing node, duplicate edge, missing edge, missing endpoint, reserved property, structured search, full-text search, vector text search, traversal query, and combined search plans.
- No SQL or Grafeo fixture is required.

## Validation Commands

- `CMD-BE-LPG-UNIT` proves domain validation, port calls, duplicate/missing mapping, endpoint consistency, and typed query contracts.
- `CMD-BE-LPG-GUARD` proves no forbidden HTTP, Grafeo, raw query, vector input, or `inheritable` surface is added.
- `CMD-BE-CHECK` proves backend compile through Makefile.
- `CMD-BE-FMT` proves formatting.
