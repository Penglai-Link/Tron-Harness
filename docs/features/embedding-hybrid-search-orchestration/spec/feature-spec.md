# Feature Spec: Embedding And Hybrid Search Orchestration

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: embedding-hybrid-search-orchestration
- feature_name: Embedding And Hybrid Search Orchestration
- status: approved_with_defaults
- owner: clouds3n
- source_request: implement PRD feature `Backend Graph Storage - Embedding And Hybrid Search Orchestration` after scope, LPG contract, and embedded Grafeo adapter are proven
- main_agent: lead agent on `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/base`
- base_branch: `feature/clouds3n/graph-storage-foundation/base`
- base_commit: `403c0b3be80273586479bd74f30878e0679f13cf`
- delivery_type: new_feature
- dependency_features: graph-scope-and-isolation-kernel complete; graph-store-lpg-domain-contract complete; embedded-grafeo-adapter-foundation complete; existing llm runtime and model management foundations complete
- dependency_contracts: `GraphService`; `GraphStore`; `GraphSearchQuery`; `GraphNodeWrite`; `GraphEdgeWrite`; `ResolvedGraphScope`; `LlmRuntimeService`; `ModelService::embedding_runtime_model_config`; graph and llm error taxonomies; backend runtime wiring in `tron-backend/src/app.rs`; PRD graph storage feature inventory

## Requirement

Tron needs one backend-only orchestration slice that derives vectors through the existing LLM runtime, attaches those vectors to graph writes without exposing caller-supplied vectors, and executes scoped hybrid graph search that can combine structured filters, full-text clauses, vector clauses, and traversal constraints in one typed request. The first version must keep graph scope isolation intact, fail fast when vector search or vector-indexed writes cannot obtain embeddings, and keep all vector material and Grafeo details behind internal Rust boundaries.

## Product Source Slices

- selected_feature: `Feature: Backend Graph Storage - Embedding And Hybrid Search Orchestration`
- selected_page: `Page: Backend API - Internal Graph Service`
- selected_interactions: `Interaction: Backend API - Store Graph Record`; `Interaction: Backend API - Search Graph`
- selected_rules: `Rule: Backend Graph Storage - Internal Embedding Ownership`; `Rule: Backend Graph Storage - Hybrid Query Composition`; `Rule: Backend Graph Storage - Raw Query Shield`
- selected_exceptions: `Exception: Backend Graph Storage - Embedding Unavailable`; `Exception: Backend Graph Storage - Embedding Provider Failure`; `Exception: Backend Graph Storage - Storage Failure`
- selected_acceptance_source: `Acceptance: Backend Graph Storage - Embedding And Hybrid Search Orchestration`
- selected_workspace: new `docs/features/embedding-hybrid-search-orchestration`
- feature_inventory_summary: this feature consumes the completed scope kernel, typed LPG contract, and embedded Grafeo adapter to add real vector ownership and hybrid query execution
- shared_requirement_summary: callers still provide text-only graph contracts; vectors are derived internally and passed only to prepared store requests
- dependency_map_summary: scope kernel plus LPG contract plus Grafeo adapter feed this feature; future knowledge-graph, memory, retrieval, app, task, and agent modules depend on this feature for vector search behavior
- implementation_order_summary: preserve typed graph contracts first, derive embeddings second, prepare vector-enriched adapter requests third, and only then execute hybrid search inside the resolved Grafeo scope

## Scope Boundary

This feature owns graph embedding orchestration, graph search preparation, prepared vector-aware store requests, and Grafeo adapter vector persistence or scoring support. It adds no HTTP route, no frontend integration, no caller-supplied raw vector field, no public rerank API, and no public raw query surface. Ontology persistence remains out of scope for vector orchestration.

## Assumptions

- The current graph scope kernel remains the only source of namespace, named graph, current workspace, and visible lineage rules.
- `LlmRuntimeService` and workspace-aware embedding model resolution already exist and are authoritative for embedding derivation.
- Current caller-facing graph commands can stay text-first; model override may be typed metadata, but raw vectors remain internal-only.
- The high-level `grafeo` crate in this repository does not yet expose a stable first-class vector-index API through the paths already validated here, so the first delivery may compute vector similarity inside the adapter as long as it remains scoped to one named graph and the resolved visible lineage.
- The first delivery may persist vectors in adapter-owned internal Grafeo fields or vector projection records that are never exposed through `GraphNode`, `GraphEdge`, or caller search DTOs.
- Graph write or search operations that require vectors must fail atomically if no embedding model resolves or the provider fails; they must not silently degrade to non-vector execution.

## In Scope

- Typed graph embedding metadata on write and search commands, including optional model override and internal vector-index request intent.
- Async or embedding-aware graph service orchestration that can call `LlmRuntimeService::embedding_call` and `embedding_call_with_chunking` before store execution.
- Prepared internal store requests that carry derived query vectors or record vectors while keeping caller-facing contracts text-only.
- Adapter-owned vector persistence for node and edge records, including chunked vector payloads for long `embedding_text` content.
- Hybrid search execution that combines structured filters, labels, full-text, traversal-derived candidates, and vector similarity in one scoped request path.
- Typed graph embedding errors mapped from model resolution failure, unavailable embedding model, invalid provider response, and provider execution failure.
- Backend tests proving embedding derivation, no-silent-degrade behavior, hybrid composition, and scope isolation under vector search.

## Out Of Scope

- HTTP controllers, request DTOs, response DTOs, route registration, or frontend graph surfaces.
- Caller-supplied raw vectors as the normal graph write or search contract.
- Rerank quality tuning or rerank-based post-processing.
- Ontology embedding, ontology vector search, or ontology rerank behavior.
- Grafeo server mode, enterprise-only vector features, or cross-root multi-graph search.
- Public REST, GraphQL, GQL, Cypher, SPARQL, Turtle, or SHACL endpoints.

## Constraints

- Any vector-dependent graph operation must start from one resolved `GraphScopeCommand` and execute only inside one resolved named graph.
- `visible_workspace_codes` remain the hard read boundary for full-text, traversal, and vector candidate sets.
- Graph callers still supply text and typed graph clauses only; vectors never become part of the stable caller contract.
- Vector search with no available embedding model returns a typed graph embedding error and does not silently run only structured or full-text clauses.
- Chunked write embeddings must be derived through `embedding_call_with_chunking`; single-text writes and query vectors must use `embedding_call`.
- Public domain and application signatures must not expose Grafeo vectors, Grafeo sessions, or raw Grafeo query strings.
- Any first-version fallback for vector scoring must stay bounded to the resolved named graph and visible lineage and must preserve typed hybrid semantics.

## Feature Model

### Actors

- Internal graph caller: backend module that creates or searches graph records.
- Graph service: validates the typed graph request, resolves scope, derives vectors, prepares the store request, and returns typed graph results.
- LLM runtime service: resolves embedding models and derives vectors.
- Grafeo graph adapter: persists internal vector projections and executes scoped hybrid search.
- Embedded Grafeo catalog: stores LPG records and adapter-owned vector payloads.

### Core Objects

- `GraphEmbeddingWriteIntent`: typed internal or caller-facing metadata that tells the graph service whether a write should derive vectors and whether chunking is required.
- `GraphEmbeddingModelOverride`: optional typed embedding model override code for a write or search request.
- `PreparedGraphNodeWrite`: adapter-facing prepared node write with caller text plus derived vectors or chunk vectors.
- `PreparedGraphEdgeWrite`: adapter-facing prepared edge write with caller text plus derived vectors or chunk vectors.
- `PreparedGraphSearchQuery`: adapter-facing prepared search request with typed filters, traversal, text clauses, and optional query vector.
- `GraphVectorChunk`: one chunk content plus its derived embedding vector.
- `GraphEmbeddingError`: typed graph error family for unavailable model or provider failure.
- `HybridGraphSearchScore`: adapter-owned fused scoring rule across vector, text, and traversal candidate constraints.

### Rules

- `embedding_text` is the only normal source text for write vector derivation; no caller raw vector field is accepted.
- A write with vector intent and short text uses one embedding call; a write marked for chunking uses chunked embedding calls.
- A search with `vector_text` always derives one query vector before the store adapter runs.
- Hybrid search narrows candidates by scope first, then applies traversal and typed filters, then fuses vector and text relevance inside the same request path.
- If vector similarity is requested, no weaker fallback query is allowed when embedding derivation fails.
- Vector payloads remain adapter-owned internal state and are not returned through `GraphNode`, `GraphEdge`, or public graph search DTOs.

### State Transitions

- caller submits typed graph write or search -> graph service resolves scope
- vector intent present -> graph service resolves embedding model and derives vector or chunk vectors
- prepared request built -> adapter persists vector-owned internal fields or executes hybrid vector scoring
- vector or provider failure -> graph service returns typed embedding error without store mutation
- successful hybrid search -> typed graph search hits return with fused score and no vector leakage

## Page And Module Surface

- page_surface: no frontend page and no HTTP route in this version
- internal_surface: Rust modules under `tron-backend/src/domain/graph`, `tron-backend/src/application/graph`, and `tron-backend/src/adapters/outbound/persistence`
- persistence_surface: existing Grafeo graph adapter grows vector-aware prepared request support; ontology adapter is unchanged
- likely_files_to_add: `tron-backend/src/application/graph/embedding.rs`; `tron-backend/src/application/graph/embedding_tests.rs`; `tron-backend/tests/graph_embedding_hybrid_contract.rs`
- likely_files_to_update: `tron-backend/src/domain/graph/lpg.rs`; `tron-backend/src/domain/graph/error.rs`; `tron-backend/src/application/graph/service.rs`; `tron-backend/src/application/graph.rs`; `tron-backend/src/adapters/outbound/persistence/grafeo_graph_store.rs`; `tron-backend/src/adapters/outbound/persistence/grafeo_graph_records.rs`; `tron-backend/src/app.rs`
- test_surface: graph service unit tests with fake llm runtime and fake store plus Grafeo-backed contract tests for vector persistence and scoped hybrid search

## Repository Findings

- `LlmRuntimeService` already exposes async `embedding_call` and `embedding_call_with_chunking`, and model management already resolves embedding runtime models per workspace.
- `GraphService` is currently synchronous and cannot yet await LLM runtime embedding calls.
- `GraphSearchQuery` already models `vector_text`, but `GrafeoGraphStore::search` currently returns `GraphError::UnsupportedStoreBehavior` when `vector_text` is present.
- Current Grafeo adapter search path already loads scoped nodes and edges into memory for filtering and traversal, so first-version vector similarity can be added without widening scope.
- No `docs/features/embedding-hybrid-search-orchestration/` workspace existed before this feature spec was added.

## Open Decisions

- None. Defaults adopted for execution: keep caller contracts text-only, add typed model override metadata where needed, make graph embedding orchestration async, persist adapter-owned vector payloads internally, and compute first-version cosine similarity inside the scoped Grafeo adapter when a stable high-level native vector index path is not yet proven in this repository.
