# Database Design: RAG 管理配置 API 接入

## Database Decision

- status: approved
- approved_by: lead-agent
- approved_at: 2026-06-10

Phase 1 added management-side corpus topology persistence and the minimal SQLite runtime evolution needed to honor an independently bound `vectorTable`. The current evolution extends that design into role-aware storage so `source`, `textUnit`, `fulltext`, and `vector` can bind to independent physical targets.

## Existing Schema Impact

Current `rag_corpora` storage uses:

- `capabilities_json`
- `filter_schema_json`
- `field_mapping_json`
- `storage_binding_json`

Current `rag_profiles` storage uses:

- `config_json`
- `filters_json`
- `degrade_policy`

Runtime code still reads `storage_binding_json` and profile storage directly.

## Proposed Schema

### rag_corpora

Add:

- `storage_topology_json TEXT NULL`

Meaning:

- stores the frontend-approved target-first management topology
- becomes the source of truth for management UI read/write
- may be null for legacy seeded corpora until first management edit or migration backfill

Keep:

- `storage_binding_json TEXT NOT NULL`

Meaning:

- remains the runtime compatibility projection used by current ingest/search/debug/retry code
- may now include optional role fields such as `fulltextTable`, `vectorTable`, graph fulltext/vector namespace, and graph fulltext/vector node labels so management saves can preserve independent retrieval carriers without adding more SQL columns

No new column is required for `metadataGroups`; they remain preserved inside `capabilities_json`.

### rag_profiles

No new table or column is required in phase 1 if:

- ingest profile management DTOs can be normalized into current `config_json`
- retrieval profile management DTOs can be normalized into current `config_json`, `filters_json`, and `degrade_policy`

If implementation proves that current profile columns cannot round-trip the management DTO safely, revisit schema expansion in a follow-up evolution rather than silently dropping configuration.

## Relationships And Cardinality

- One `rag_corpora` row owns one management storage topology and one runtime compatibility binding.
- One corpus can be referenced by zero or more ingest profiles and zero or more retrieval profile routes.
- One retrieval profile can reference multiple corpora.
- `metadataGroups` remain embedded inside corpus capability storage and keep one-to-one cardinality with the corpus row.

## Persistence Rules

- On corpus read:
  - if `storage_topology_json` exists, decode and expose it
  - else derive a management topology from legacy `storage_binding_json`
- On corpus save:
  - validate management topology
  - normalize missing legacy `fulltext` bindings to the `textUnit` target
  - write `storage_topology_json`
  - project supported combinations into `storage_binding_json`, including optional `fulltextTable` and `vectorTable` compatibility data when retrieval carriers differ from `textUnitTable`
  - preserve existing `metadataGroups` unless the request explicitly includes a replacement value

- On runtime read:
  - legacy readers continue consuming `storage_binding_json`
  - SQLite runtime execution uses `textUnitTable` for canonical text rows, optional `fulltextTable` for fulltext carrier rows/FTS sidecars, and optional `vectorTable` for vector carrier rows/vec sidecars when present
  - fulltext and vector search join carrier rows back to `textUnitTable` through `text_unit_code` so runtime response contracts remain stable while storage becomes independent
  - Graph-backed fulltext/vector carrier targets route retrieval to graph adapters while canonical SQLite source/text-unit rows remain the hydration source

- On profile read:
  - normalize current storage into the frontend form DTO
- On profile save:
  - validate against catalog/options and filter schema
  - write back to current runtime-readable storage fields

## Migration Notes

- Migration must be idempotent and safe on existing seeded databases.
- Backfill of `storage_topology_json` for existing rows may be done lazily on read/write if a one-shot SQL backfill is not practical.
- Tests must cover both legacy rows without `storage_topology_json` and newly saved rows with it.

## Migration Plan

- Apply `tron-backend/migrations/20260610_0001_add_rag_management_storage_topology.sql` idempotently on existing databases.
- Preserve lazy backfill behavior for legacy rows whose detail reads still need to derive topology from `storage_binding_json`.
- Keep runtime readers on `storage_binding_json` while management reads/writes adopt `storage_topology_json`.

## Query And Performance Plan

- Corpus and profile list/detail queries should stay on the existing lightweight config tables and avoid touching runtime source/text-unit data.
- Catalog endpoints should be in-memory or config-backed and must not require heavy runtime repository scans.
- Compatibility projection should be computed from one corpus payload at save/verify time and not fan out into ingest/search repositories.

## Data Governance

- Management edits must preserve stored `metadataGroups` unless explicitly replaced by a request payload.
- Unsupported topology combinations must fail validation instead of persisting partially projected runtime data.
- Legacy runtime readers remain supported while the new management topology column is rolled out.

## Test Data And Fixtures

- Reuse existing seeded corpora and profiles where possible.
- Add backend test fixtures for:
  - legacy corpus rows without `storage_topology_json`
  - rows with supported projected topology
  - rows with unsupported topology submitted through verify/save
  - retrieval profiles with multiple corpora
  - role-aware rows with independent `fulltextTable`
  - mixed SQLite canonical rows with Graph fulltext/vector carriers

## Validation Commands

- `make be-test`
- `make be-check`
- `cargo test --manifest-path tron-backend/Cargo.toml rag_`
