# API Contract: Embedding And Hybrid Search Orchestration

## API Overview

This feature adds no HTTP endpoint and no public vector API. It extends only internal Rust graph contracts and graph service orchestration so graph writes and searches can derive vectors through the existing LLM runtime and pass those vectors only to internal prepared store requests.

## Endpoints

- none
- No backend route is added for `/api/v1/knowledge-graph`.
- No backend route is added for `/api/v1/memory/fragments`.
- No public REST, GraphQL, GQL, Cypher, SPARQL, Turtle, or raw vector endpoint is introduced.

## Internal Rust Contract

| type | layer | role |
| --- | --- | --- |
| `GraphNodeWrite` / `GraphEdgeWrite` | domain | caller-facing typed graph writes with text only |
| `GraphSearchQuery` | domain | caller-facing typed hybrid search request with `vector_text`, not raw vectors |
| `PreparedGraphNodeWrite` | application | adapter-facing write with derived vector payloads |
| `PreparedGraphEdgeWrite` | application | adapter-facing edge write with derived vector payloads |
| `PreparedGraphSearchQuery` | application | adapter-facing search plan with optional query vector |
| `GraphService` | application | async embedding-aware graph orchestration |
| `LlmRuntimeService` | application | embedding derivation dependency |
| `GrafeoGraphStore` | outbound adapter | scoped vector persistence and hybrid search implementation |

Suggested Rust shape:

```rust
pub struct PreparedGraphNodeWrite {
    pub command: GraphNodeWrite,
    pub embedding_model_code: Option<String>,
    pub query_vector: Option<Vec<f32>>,
    pub chunk_vectors: Vec<GraphVectorChunk>,
}

pub struct PreparedGraphSearchQuery {
    pub query: GraphSearchQuery,
    pub embedding_model_code: Option<String>,
    pub query_vector: Option<Vec<f32>>,
}
```

## Auth And Permissions

- This feature adds no new HTTP authentication or authorization boundary.
- Future internal callers must continue to authorize before calling graph services.
- Vector resolution follows the existing workspace-aware embedding model resolution path and must not bypass graph scope visibility.

## Error Contract

| error | condition |
| --- | --- |
| typed graph embedding unavailable error | vector write or search is requested but no embedding model resolves |
| typed graph embedding provider failure error | runtime provider rejects, times out, or returns invalid vector data |
| `StorageFailure` | Grafeo persistence or hybrid search execution fails after embeddings are prepared |
| `UnsupportedStoreBehavior` | adapter cannot satisfy a required vector or hybrid semantic with the configured storage behavior |
| existing graph validation errors | request never reaches vector orchestration because command validation fails first |

## Frontend Backend Boundary

- No frontend code changes are required to consume this feature.
- No frontend mock route is converted to a backend route in this slice.
- Browser evidence is not required because this feature remains backend-internal.
