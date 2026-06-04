# Backend Result: RAG Storage Backend Fusion

## Evidence

- AC-RSBF-01 | status: completed | red: storage-profile worker focused contract failed before implementation because `RagStorageBackend` was not exported and `RagStorageBinding` had no `backend` field | green: explicit `SQLITE` / `GRAPH` backend parsing and backend-first graph repository dispatch implemented | verify: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_service_contracts -- --nocapture` passed, 7/7 and 15/15; `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture` passed, 4/4.
- AC-RSBF-02 | status: completed | red: storage-profile worker added `skill-memory-project-fusion-v1` assertions in the same focused contract batch; the first RED run stopped at the missing backend compile gate | green: migration seeds a retrieval profile with `skill-graph`, `memory-card`, and `project-doc` routes | verify: profile contract assertions pass in `rag_service_contracts`; search coverage remains green.
- AC-RSBF-03 | status: completed | red: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts request_filters_match_each_route_filter_schema -- --nocapture` failed before implementation with `SearchFilterInvalid` on mixed corpus request filters | green: request filters now match each route corpus schema and ignore absent request fields | verify: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture` passed, 21/21.
- AC-RSBF-04 | status: completed | red: pre-change graph RAG search built plain prepared graph queries even when route embeddings existed; after merge, the graph contract fixture also failed with missing `query_embedding_model_code`, proving the new field had to flow through route construction | green: `RagSearchRuntime::embed` returns both embedding vector and model code, and graph RAG search builds `PreparedGraphSearchQuery` with `semantic_projection` whenever a route query carries an embedding | verify: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture` passed, 4/4; unit `graph_route_query_embedding_becomes_prepared_semantic_projection` passes inside `make be-test`.
- AC-RSBF-05 | status: completed | red: pending evidence file and final graph contract fixture failed before closeout | green: final fixture, docs, and worker merges completed | verify: focused commands and full `make be-test` passed.

## Commands

- `make be-check` passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_service_contracts -- --nocapture` passed: `rag_foundation_schema` 7/7, `rag_service_contracts` 15/15.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture` passed: 21/21.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture` passed: 4/4.
- `cargo fmt --manifest-path tron-backend/Cargo.toml --check` passed.
- `make be-test` passed.

## Implementation Summary

- Added `RagStorageBackend` with only two current values: `SQLITE` and `GRAPH`.
- Kept dispatch backend-first: non-graph corpora delegate to the SQLite adapter; graph corpora delegate to the RAG graph adapter path.
- Kept layout as a graph-internal adapter selector: current `GRAPH + GRAPH_SINGLE_NODE` is implemented; future `GRAPH + GRAPH_TEXT_UNIT_SUBGRAPH` should add a new RAG graph layout adapter instead of changing store APIs.
- Seeded `skill-memory-project-fusion-v1` so skill graph, memory, and project document corpora can be fused through configuration.
- Changed request-level filter normalization so absent fields are ignored per corpus schema, while profile and route config filters remain strict.
- Passed query embedding model code and vector values into graph prepared search queries through `semantic_projection`.

## Boundaries

- Store APIs were not changed; work stayed inside the current RAG module and existing graph prepared-search contract.
- The current graph vector path wires semantic projection into graph search; deeper graph engine behavior is limited to the existing graph store capabilities.
- Unsupported graph layouts now fail explicitly with `StorageBindingInvalid` instead of silently falling back to SQLite.
- Browser evidence: clicks=n/a, inputs=n/a, network=n/a, feedback=n/a, artifacts=n/a; this was backend-only Rust contract work.
