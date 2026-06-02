# SQLite Extension Search

## Feature Identity And Context

This feature upgrades backend RAG storage search from prototype-only SQLite behavior to extension-backed retrieval while preserving the existing simple strategies.

The existing RAG search path keeps its HTTP request and response contract. Retrieval profiles may continue to use the legacy LIKE full-text route and in-process vector cosine route. New storage retrievers are added for SQLite FTS5 with the sqlite-simple tokenizer and sqlite-vec cosine search.

## Scope Boundary

In scope:

- Shared backend SQLite extension capability detection and per-connection loading.
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
- Moving business attributes out of the existing text-unit tables.

## Design

The canonical RAG text-unit tables remain the business source of truth. Extension-backed indexes are sidecars joined by `rowid = text_unit.id`.

Full-text indexes use FTS5 external-content virtual tables over `search_text`. The preferred tokenizer is sqlite-simple `simple`; if extension loading is disabled or unavailable, the new extension-backed retriever is unavailable rather than silently becoming the legacy LIKE implementation.

Vector indexes use sqlite-vec `vec0` virtual tables. Because vec0 requires a fixed dimension per table, the migration creates known dimension sidecars and a registry table describing which corpus/table/dimension bindings are active. The first implementation supports the existing dimensions used by tests and seed data, while keeping the registry shape extensible.

The shared package owns platform detection, native asset extraction, loadable extension setup, and capability probing. RAG repositories consume that package during connection setup and migration.

## Data Flow

On startup, backend configuration creates a shared SQLite extension context. Each RAG SQLite connection asks the context to prepare the connection, then applies migrations. Migrations create sidecar index tables and registry rows.

During ingest or retry persistence, the business tables are written first. The repository then mirrors eligible rows into FTS and vec sidecars in the same storage flow. A rebuild routine can backfill sidecars from existing `INDEXED` rows.

During search, legacy retrievers keep their existing path. Extension-backed full-text search uses `MATCH` and BM25 score. Extension-backed vector search uses `vec_distance_cosine` and returns storage score. Application ranking consumes storage score for new retrievers, then continues existing route fusion, deduplication, optional rerank, event publication, and context packing.

## Error Handling

Extension-backed retrievers fail with route-unavailable behavior when their required capability is absent. Legacy retrievers remain available.

Schema migration must be idempotent. Rebuild must skip rows whose embedding dimension has no active vec table and must report the skipped count through diagnostics or test-visible result data.

## Testing

Backend tests cover capability detection, schema creation, sidecar rebuild, legacy retriever preservation, extension-backed full-text search, extension-backed vector search, and dimension mismatch handling.
