# Database Design: RAG Multi-Corpus RRF And Topic Strategy

## Database Decision

- status: not_applicable
- reason: The feature reuses existing RAG corpus text-unit tables, source tables, and vec0 sidecar indexes.

## Existing Schema Impact

- No schema changes.
- Existing text-unit `embedding` columns remain the source for application-level MMR.
- Existing vec0 tables remain the route retrieval index.

## Proposed Schema

Not applicable.

## Relationships And Cardinality

- Retrieval profiles can reference multiple corpus IDs through existing profile config.
- Route hits keep per-route rank metadata in memory only.
- Dedup groups are application-level candidate groups and are not persisted.

## Migration Plan

No migration required.

## Query And Performance Plan

- SQLite route queries continue to limit candidates per route.
- The projection includes stored text-unit embeddings when the physical table exposes them.
- MMR operates only on the configured candidate window.

## Data Governance

- Stored embeddings are used in memory for ranking only.
- Search traces do not store raw vectors or provider responses.

## Test Data And Fixtures

- Contract tests seed memory and project-doc corpora through existing ingest services.
- Project-doc fixtures use a related but distinct vector so MMR can verify topic diversity.

## Validation Commands

- `make be-test`
- `make be-check`
