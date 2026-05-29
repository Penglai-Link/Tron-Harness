# Database Design: Embedded Grafeo Adapter Foundation

## Database Decision

- status: required
- mandatory_database_change: yes
- rationale: this feature is the first production graph persistence slice and must store LPG and ontology data in embedded Grafeo instead of fake in-memory stores.

## Existing Schema Impact

- Existing SQLite-backed workspace tables remain the authoritative source for workspace lineage and scope resolution.
- Existing LPG and ontology Rust contracts remain unchanged at the caller boundary.
- No previous Grafeo catalog or graph schema exists in the repository today.

## Proposed Schema

### Embedded Catalog

- one embedded Grafeo catalog
- one configurable local catalog path, separate from the existing SQLite `database_url`
- no Grafeo server deployment
- no Grafeo enterprise-only feature dependency

### Schema And Named Graph Mapping

| source | Grafeo mapping |
| --- | --- |
| `ResolvedGraphScope.schema` | Grafeo schema name |
| `ResolvedGraphScope.named_graph` | Grafeo named graph within that schema |
| `ResolvedGraphScope.current_workspace_code` | required write property `workspaceCode` |
| `ResolvedGraphScope.visible_workspace_codes` | lineage filter for reads, search, and traversal |
| `storage_dimensions.appCode` | required write property `appCode` |
| `storage_dimensions.userCode` | required write property `userCode` |

### Required Stored Properties

- `namespace`
- `workspaceCode`
- `appCode`
- `userCode`
- `createdAt`
- `updatedAt`

### Graph Storage Shape

- LPG nodes store stable `node_code`, labels, typed business properties, optional `full_text`, optional `embedding_text`, and required isolation properties.
- LPG edges store stable `edge_code`, `source_node_code`, `target_node_code`, `relationship`, typed business properties, optional text fields, and required isolation properties.
- Ontology resources store typed class, property, domain, range, constraint, and shape data using Grafeo-native ontology or RDF capability without exposing raw RDF to callers.

## Relationships And Cardinality

- One embedded catalog contains many namespace schemas.
- One namespace schema contains many root-workspace named graphs.
- One named graph contains many LPG nodes and edges plus many ontology resources in the same root-workspace boundary.
- One read scope can see one or more `workspaceCode` values, but all of them remain inside one root-workspace named graph.

## Migration Plan

- No SQL migration is required.
- Grafeo catalog bootstrap runs on first adapter initialization.
- Schema, named-graph, and index creation must be idempotent.
- No backfill is required because the graph store has not previously persisted production graph data in this repository.

## Query And Performance Plan

- Create or verify indexes for stable graph identifiers and required isolation properties.
- Read, search, and traversal operations must always bind to one named graph first, then apply lineage and optional app or user filters.
- Full-text and hybrid search features may use Grafeo-native index support only when the capability preserves the typed contract semantics.
- If a requested capability is unavailable, return typed unsupported behavior instead of widening scope or performing an unbounded scan that violates the contract.

## Data Governance

- `__NONE__` remains the only allowed persisted absent-dimension sentinel.
- Required graph isolation property names remain reserved and unavailable to dynamic business properties.
- Raw query strings stay inside the adapter; they are never caller input and never part of public typed contracts.
- Graph and ontology storage logs must avoid dumping full record payloads on storage failure paths.

## Test Data And Fixtures

- Adapter tests use temporary embedded Grafeo storage roots.
- Fixtures should cover multiple namespaces, multiple root workspaces, lineage reads, duplicate node, duplicate edge, missing node, missing edge, ontology duplicate or missing resource, and unsupported capability mapping.
- Tests should prove one operation never crosses root named-graph boundaries.

## Validation Commands

- `CMD-BE-GRAFEO-CONTRACT` proves embedded adapter bootstrap, scope isolation, graph persistence, ontology persistence, and typed error mapping.
- `CMD-BE-GRAFEO-GUARD` proves no graph HTTP route was added and architecture rules still pass.
- `CMD-BE-CHECK` proves backend compiles through the project Makefile.
- `CMD-BE-TEST` proves the backend test suite still passes with embedded Grafeo present.
