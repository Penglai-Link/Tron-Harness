# Database Design: RAG Pipeline DAG Runtime

## Database Decision

- status: approved

Reuse existing `rag_profiles.config_json` to store the new Pipeline DAG payload. No new table is required for the first version.

## Existing Schema Impact

Existing RAG corpus and profile tables remain in place. Profile rows continue to identify `profile_id`, profile type, status, corpus ids, required source version, config JSON, filters, and degrade policy.

## Proposed Schema

No schema migration is required. The proposed `config_json` shape includes:

```json
{
  "pipeline": {
    "mode": "DAG",
    "nodes": [],
    "edges": []
  }
}
```

## Relationships And Cardinality

One profile belongs to one primary corpus and can reference multiple corpora. One profile contains one executable Pipeline DAG. One Pipeline DAG contains many nodes and edges.

## Migration Plan

The first implementation stores new profile saves in the new JSON shape. Existing seeded profile compatibility is handled at the application projection or compiler boundary so startup and runtime flows do not fail.

## Query And Performance Plan

Profile reads continue to load a single JSON blob by profile id. DAG validation is in-memory and bounded by the small number of configured profile nodes.

## Data Governance

Pipeline config must not store secrets or provider raw responses. Stage diagnostics and warnings belong to runtime response or existing diagnostics paths.

## Test Data And Fixtures

Tests use seeded RAG corpora and mock profile payloads with branch and merge edges. Frontend mock fixtures must include pipeline catalog data and runtime stage results.

## Validation Commands

- `cargo test --manifest-path tron-backend/Cargo.toml --lib rag`
- `make fe-test TEST=tron-frontend/tests/rag-profile-form-pages.spec.ts`
