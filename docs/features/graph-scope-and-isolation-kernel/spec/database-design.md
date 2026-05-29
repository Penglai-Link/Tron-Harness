# Database Design: Graph Scope And Isolation Kernel

## Database Decision

- status: not_applicable
- mandatory_database_change: no
- rationale: this feature produces validated graph storage scope values only; no SQL table, migration, Grafeo catalog, schema, named graph, index, node, edge, or RDF resource is created here.

## Existing Schema Impact

- Existing SQLite-backed workspace tables remain the authoritative source for workspace details and lineage.
- Existing graph-like frontend mock endpoints do not receive backend tables in this slice.
- No current persistence schema is modified.

## Proposed Schema

### Phase 1: Scope Kernel

- no SQL table
- no SQL migration
- no Grafeo catalog creation
- no Grafeo schema creation
- no Grafeo named graph creation
- no Grafeo index creation

### Future Adapter Phase

The later embedded Grafeo adapter feature will consume this feature's resolved scope fields:

| scope field | future Grafeo use |
| --- | --- |
| `schema` | maps from namespace to Grafeo schema |
| `named_graph` | maps from root workspace code to Grafeo named graph |
| `visible_workspace_codes` | filters reads to root-to-current lineage |
| `storage_dimensions.workspaceCode` | stores the current workspace code on writes |
| `storage_dimensions.appCode` | stores concrete app dimension or `__NONE__` |
| `storage_dimensions.userCode` | stores concrete user dimension or `__NONE__` |

## Relationships And Cardinality

- One namespace resolves to one future Grafeo schema within the single embedded catalog.
- One root workspace resolves to one future Grafeo named graph within the namespace schema.
- One current workspace resolves to one write `workspaceCode`.
- One read scope contains one or more visible workspace codes from root to current workspace.
- App and user dimensions are optional at the command boundary but one concrete storage value each after normalization.

## Migration Plan

- No migration is required for this feature.
- No existing data backfill is required.
- Future Grafeo adapter migration and index setup belong to the embedded adapter feature, not this scope kernel.

## Query And Performance Plan

- Scope resolution performs one workspace detail lookup through `WorkspaceService::get_workspace`.
- Scope derivation uses the returned workspace path in memory and should remain O(path length).
- No database query is introduced outside the existing workspace service call.
- Later store adapters should treat `visible_workspace_codes`, `appCode`, and `userCode` as indexed filter inputs.

## Data Governance

- Scope errors must not log dynamic graph record payloads.
- The sentinel `__NONE__` is reserved for storage absence and must not be accepted as caller-supplied app or user code.
- Required future storage property names are reserved: `namespace`, `workspaceCode`, `appCode`, `userCode`, `createdAt`, and `updatedAt`.
- No `inheritable` flag or equivalent persistence switch is introduced.

## Test Data And Fixtures

- Unit tests should use in-memory workspace detail fixtures with root and child path examples.
- Fixtures should cover active workspace, archived current workspace, empty lineage, blank namespace, blank workspace, missing app/user dimensions, explicit app/user dimensions, and reserved sentinel misuse.
- No SQL fixture or Grafeo fixture is required in this feature.

## Validation Commands

- `CMD-BE-GRAPH-SCOPE-UNIT` proves scope validation, lineage mapping, sentinel normalization, and reserved property behavior.
- `CMD-BE-CHECK` proves new graph modules compile with existing backend facades.
- `CMD-BE-ARCH` proves the no-`mod.rs` and domain purity constraints still hold.
