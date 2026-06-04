# Feature Spec: RAG Structured Filter Retrieval

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

This feature evolves the existing RAG search pipeline so callers can use a Retrieval Profile for filter-only structured retrieval. The current search path is query-oriented: full-text routes use the user query, vector routes call embedding, and rerank also depends on the query. This feature adds an explicit structured filter retriever instead of letting blank full-text queries behave like list queries.

## Feature Decomposition Summary

- feature_slug: rag-structured-filter-retrieval
- feature_name: RAG Structured Filter Retrieval
- delivery_type: feature_evolution
- target_workspace: `docs/features/rag-structured-filter-retrieval`
- goal: allow RAG search callers to omit `query` when the selected profile has a structured-filter route, returning active scoped records constrained only by filters and source version.
- in_scope: SQLite structured-filter retriever registration, profile compilation, empty-query route gating, SQLite search adapter support, HTTP validation relaxation, focused backend TDD coverage, docs and evidence.
- deferred_or_out_of_scope: GraphSingleNode structured retrieval, pagination response shape, dynamic orderBy, aggregation, frontend profile editor, new HTTP endpoint.
- dependency_features: `rag-search-context-pack`, `sqlite-extension-search`, `rag-rrf-topic-strategy`
- source_prd_headings: approved thread requirement "filter-only structured retrieval for RAG search" on 2026-06-04
- recommended_order: implement after existing RAG search route fusion and SQLite extension search because it reuses route compilation, filter merging, fusion, dedup, and result assembly.

## Feature Identity And Context

- status: approved_with_defaults
- owner: langtaoxin
- source_request: user asked to complete the approved structured-filter strategy with AC split, git worktree isolation, multi-agent support when parallelizable, TDD verification, and evidence report.
- main_agent: current-thread lead agent
- dependency_features: `rag-search-context-pack`, `sqlite-extension-search`, `rag-rrf-topic-strategy`
- dependency_contracts: `tron-backend/src/application/rag/search.rs`, `tron-backend/src/application/rag/search_strategy.rs`, `tron-backend/src/application/rag/search_validation.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`, `tron-backend/src/adapters/inbound/http/rag/*`, `tron-backend/tests/rag_search_contracts.rs`, `tron-backend/tests/rag_search_api.rs`

## Requirement

- user_story: As a RAG caller, I need to retrieve records by declared filter fields even when I do not have a natural-language question.
- intent: keep query-based RAG behavior intact while adding a first-class route type for structured, filter-only retrieval.
- outcome: a request with no `query` succeeds only when at least one structured-filter route can run; full-text, vector, embedding, and rerank work are skipped in that mode.

## Product Source Slices

- brainstorming_source: current thread, "形成一个具体的可落地方案和边界信息", approved by follow-up implementation request on 2026-06-04.
- prd_feature_headings: `Feature: RAG Structured Filter Retrieval`
- prd_page_headings: `Backend API - RAG Search`
- prd_interaction_headings: `Interaction: Filter-only RAG search request`
- prd_rule_headings: `Rule: Empty query route gating`, `Rule: Structured filter route score and order`, `Rule: Existing query retrieval compatibility`
- prd_exception_headings: `Exception: Empty query without structured route`, `Exception: Unsupported graph structured route`
- prd_acceptance_headings: `Acceptance: Structured filter route`, `Acceptance: Empty query behavior`, `Acceptance: HTTP contract compatibility`

## Scope Boundary

### Assumptions

- Existing Retrieval Profiles already support explicit `routes` with per-route `filters`, `topK`, and `weight`.
- Existing filter merging and filter schema validation remain the authority for profile, route, and request filters.
- The first implementation targets SQLite-backed corpora only.
- Existing search response shape remains unchanged.

### In Scope

- Register `STRUCTURED_FILTER_SQLITE_V1` as an enabled retriever strategy.
- Compile structured-filter routes through existing Retrieval Profile route parsing.
- Allow application and HTTP search requests to omit or blank `query`.
- Treat blank `query` as structured-only mode.
- Skip full-text routes, vector routes, embedding calls, and rerank calls in structured-only mode.
- Return `RAG_SEARCH_REQUEST_INVALID` when structured-only mode has no executable structured route.
- Add SQLite search support that applies active status, source version, scope, filters, stable ordering, and limit.
- Assign deterministic route score so existing ranking, fusion, dedup, context pack, and return fields continue to work.
- Preserve query-present full-text/vector behavior.

### Out Of Scope

- GraphSingleNode structured-filter retriever.
- New endpoint or pageable response body.
- User-configurable `orderBy`.
- Aggregation, count-only queries, or faceting.
- Frontend UI for profile configuration.
- Search over workspace ancestor scopes.
- Automatic fallback from blank query to full-text list behavior.

### Constraints

- Use Makefile commands first for backend build and tests.
- Preserve the project uniform response body.
- Do not weaken existing filter-schema validation.
- Do not send blank query to embedding or rerank providers.
- Do not use Graph search as an implicit structured-filter implementation in the first version.

## Feature Model

### Retriever Strategy

- strategy_id: `STRUCTURED_FILTER_SQLITE_V1`
- kind: `StructuredFilter`
- storage_retriever: `STRUCTURED_FILTER_SQLITE_V1`
- query dependency: none
- route availability: SQLite-backed corpus search adapter only

### Empty Query Mode

An empty query is one where `request.query.trim().is_empty()`.

Behavior:

- structured-filter routes execute.
- full-text routes are skipped.
- vector routes are skipped.
- query embedding is not created.
- rerank is not called.
- warnings are not emitted for intentionally skipped query-dependent routes.
- if no structured-filter route executes, the request is invalid.

### Query Present Mode

When `query` is non-empty, existing full-text and vector behavior stays stable. Structured-filter routes may also execute if configured, with deterministic score and route weight.

### Structured Route Result Semantics

- score: `1.0 * route.weight`
- default ordering: `source_code ASC, ordinal ASC`
- filtering: active source/text unit, fixed source version, allowed scopes, route/profile/request filters
- limit: existing route limit calculation applies
- output: existing `CandidateHit` and optional `ContextPack`

## API Surface

- endpoint: `POST /api/v1/rag/search`
- request change: `query` becomes optional for profiles that can run structured-filter routes.
- response change: none.
- error behavior: blank query without a runnable structured-filter route returns `RAG_SEARCH_REQUEST_INVALID`.

## Page And Module Surface

### Backend API - RAG Search

- route: `POST /api/v1/rag/search`
- request: existing JSON body with optional `query` in structured-only mode
- response: existing uniform envelope and `RagSearchResponse`
- auth: unchanged authenticated user and workspace access guard

### Rust Modules

- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_strategy.rs`
- `tron-backend/src/application/rag/search_profile.rs`
- `tron-backend/src/application/rag/search_validation.rs`
- `tron-backend/src/application/rag/search_ranking.rs`
- `tron-backend/src/application/rag/strategy_registry.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/adapters/inbound/http/rag/request.rs`
- `tron-backend/src/adapters/inbound/http/rag/handler.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_search_api.rs`

## Repository Findings

- `RagSearchRequest.query` is currently a `String` and HTTP defaults missing query to `""`.
- `validate_request` and HTTP `has_required_search_request_fields` currently reject blank query.
- `execute_routes` currently embeds before route execution when any vector route exists.
- `rank_route_rows` filters out non-positive scores, so structured-filter rows need positive scoring.
- SQLite full-text behavior can accidentally return filter-only results when query terms are empty; this must be avoided at application route-gating level.
- GraphBacked RAG search already treats graph full-text query as optional in some paths, so it must remain out of scope for this first retriever.

## Open Decisions

- None.
