# SQLite Extension Search

## Feature Identity And Context

This feature upgrades backend RAG storage search from prototype-only SQLite behavior to extension-backed retrieval while preserving the existing simple strategies.

The existing RAG search path keeps its HTTP request and response contract. Retrieval profiles may continue to use the existing full-text and vector retriever IDs, but SQLite storage now reads search payload from reusable sidecar index tables instead of index payload columns on the business text-unit tables. New storage retrievers are added for SQLite FTS5 with the sqlite-simple tokenizer and sqlite-vec cosine search.

## Scope Boundary

In scope:

- Shared backend SQLite extension platform detection and per-connection loading.
- Native sqlite-simple and sqlite-vec resources for supported local platforms.
- RAG memory and project document sidecar index schema for FTS5 and vec0.
- Rebuild/synchronization support from existing text-unit business tables into sidecar indexes.
- New retriever IDs that use SQLite extension-backed scoring.
- Focused backend tests proving old and new search paths coexist.

Out of scope:

- Changing HTTP API request or response shapes.
- Replacing existing legacy retriever IDs.
- Adding frontend behavior.
- Changing model embedding generation, rerank policy, or context pack assembly.
- Changing business attributes stored by the existing text-unit tables.

## Design

The canonical RAG text-unit tables remain the business source of truth and do not store full-text or vector index payload columns. Extension-backed indexes are sidecars joined by `rowid = text_unit.id`.

Full-text indexes use `{text_unit_table}_fts` FTS5 sidecars with the contract `rowid -> text_unit.id` and `search_text TEXT`. The preferred tokenizer is sqlite-simple `simple`; if extension loading is disabled or unavailable, full-text retrievers are unavailable at route level rather than silently reading stale main-table fields.

Vector indexes use `{text_unit_table}_vec_d4096` sqlite-vec `vec0` sidecars with the contract `rowid -> text_unit.id` and `embedding float[4096]`. The schema does not maintain separate vector registry rows.

The shared package owns platform detection, native asset extraction, and loadable extension setup. RAG repositories consume that package during connection setup and use current-connection probes or sidecar table existence checks when deciding whether extension-backed routes are available.

## Data Flow

On startup, backend configuration creates a shared SQLite extension context. Each RAG SQLite connection asks the context to prepare the connection, then applies migrations. Migrations create sidecar index tables only; extension capability and vector registry state is not persisted or cached.

During ingest or retry persistence, the business tables are written first. The repository then writes full-text and vector payload into FTS and vec sidecars in the same storage flow. A rebuild routine can backfill FTS sidecars from existing business text and can migrate legacy vector payload only when an upgraded database still has old vector columns.

During search, full-text retrievers join the FTS sidecar and vector retrievers join the fixed 4096 vector sidecar. Default memory and project document retrieval profiles use `FULLTEXT_SQLITE_FTS5_SIMPLE_V1` and `VECTOR_SQLITE_VEC0_COSINE_V1`. FTS5 search uses `MATCH` and BM25 score. Vector search uses `vec_distance_cosine` and returns storage score. Application ranking consumes storage score, then continues existing route fusion, deduplication, optional rerank, event publication, and context packing.

## Error Handling

Retrievers fail with route-unavailable behavior when their required sidecar is absent or the query embedding is not 4096-dimensional.

Schema migration must be idempotent. Legacy vector rebuild skips rows whose embedding dimension is not 4096.

## Testing

Backend tests cover connection preparation, schema creation, sidecar rebuild, legacy retriever preservation, extension-backed full-text search, extension-backed vector search, and 4096-dimension enforcement.
