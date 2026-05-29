# Database Design: Embedding And Hybrid Search Orchestration

## Database Decision

- status: required
- mandatory_database_change: yes
- rationale: this feature is the first slice that must persist adapter-owned vector payloads and evaluate vector similarity for graph records.

## Existing Schema Impact

- Existing SQLite-backed model management remains the source of embedding model resolution.
- Existing Grafeo graph nodes and edges already persist scope fields, timestamps, text fields, and business properties.
- Current Grafeo graph adapter does not yet persist any vector payload and rejects `vector_text` search.

## Proposed Schema

### Adapter-Owned Vector Payloads

The first version stores vector payloads in Grafeo as adapter-owned internal fields or projection records that are not returned through `GraphNode` or `GraphEdge`.

Required internal payloads:

- optional `embeddingModelCode`
- optional single-record vector payload for short `embedding_text`
- optional chunk vector payloads for long `embedding_text`
- optional chunk content metadata if required for debugging or replay-safe verification

### Storage Mapping

| source | Grafeo mapping |
| --- | --- |
| `GraphNodeWrite.embedding_text` | adapter-owned node vector payload |
| `GraphEdgeWrite.embedding_text` | adapter-owned edge vector payload |
| `GraphSearchQuery.vector_text` | adapter-owned prepared query vector only |
| `ResolvedGraphScope.visible_workspace_codes` | hard candidate filter before vector scoring |
| `ResolvedGraphScope.named_graph` | hard named-graph boundary for all vector persistence and scoring |

### Hybrid Search Shape

- structured filters and label filters narrow the candidate record set
- traversal clauses may narrow candidates further before vector scoring
- full-text match contributes relevance without bypassing scope
- vector similarity scores are computed only across scoped candidate records that actually have adapter-owned vectors
- final result remains `GraphSearchHit` with typed score only

## Relationships And Cardinality

- One graph record may have zero or one single-vector payload.
- One graph record may have zero or many chunk vectors.
- One vector query derives exactly one query vector per search request.
- One hybrid search result returns one fused score per visible graph node.

## Migration Plan

- No SQL migration is required.
- Grafeo adapter bootstrap remains idempotent.
- The adapter adds internal vector fields or projection records lazily when vector-indexed writes occur.
- Existing graph records without vectors remain valid and searchable by non-vector clauses.

## Query And Performance Plan

- Scope and visibility filtering happen before any vector scoring.
- First-version vector similarity may be computed in adapter memory from scoped Grafeo-loaded candidates when a stable native high-level vector index path is not yet proven in this repository.
- Chunked vectors use deterministic aggregation; first default is maximum chunk similarity per record.
- If a requested vector semantic cannot be preserved, the adapter returns a typed unsupported behavior error instead of silently weakening the query.

## Data Governance

- Raw vectors are never part of caller-facing graph write or search DTOs.
- Vector payloads remain adapter-owned internal data.
- Logging must avoid full source text, raw vector payloads, and chunk content dumps on error paths.
- Required scope properties remain mandatory on any vector-bearing record or projection.

## Test Data And Fixtures

- Fixtures should cover one scoped record with no vectors, one record with single-vector payload, one record with chunked vector payload, one vector query with traversal filter, and one unavailable-model failure case.
- Fixtures should prove vector search stays inside one named graph and visible workspace lineage.

## Validation Commands

- `CMD-BE-GRAPH-EMBED-UNIT` proves embedding derivation, chunking, and no-silent-degrade behavior.
- `CMD-BE-GRAPH-HYBRID-CONTRACT` proves Grafeo-backed hybrid search and vector persistence semantics.
- `CMD-BE-GRAPH-EMBED-GUARD` proves no graph HTTP route or caller raw vector field is added.
