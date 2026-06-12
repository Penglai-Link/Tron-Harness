# API Contract: RAG Skill Graph Corpus

## API Overview

This feature is backend-internal and profile-driven. It does not add public HTTP endpoints. Existing RAG ingest, search, debug, and retry APIs keep their uniform response body and select graph behavior only through corpus/profile configuration.

## Endpoints

### Existing RAG Ingest API - profile-driven graph storage

- purpose: callers may ingest `skill-graph` by selecting the new graph ingest profile and corpus
- auth: existing RAG auth and workspace guard
- acceptance_ids: AC-RSG-01, AC-RSG-02, AC-RSG-03, AC-RSG-04, AC-RSG-09
- parameter_passing: existing request body
- request: existing ingest DTO plus optional typed related graph payload
- response: existing uniform response envelope and ingest result shape
- client_impact: none required for old callers
- compatibility: additive; old `skill` ingest keeps current behavior

### Existing RAG Search API - profile-driven graph retrieval

- purpose: callers may search `skill-graph` by selecting the new retrieval profile
- auth: existing RAG auth and workspace guard
- acceptance_ids: AC-RSG-01, AC-RSG-05, AC-RSG-06, AC-RSG-07, AC-RSG-09
- parameter_passing: existing request body
- request: existing search DTO; relation expansion policy comes from profile config
- response: existing result hit shape with optional `metadata.relatedGraph`
- client_impact: clients can read related graph metadata when present
- compatibility: additive; top-level schema unchanged

### Existing RAG Debug And Retry APIs - layout routed operations

- purpose: graph-backed sources route to graph store instead of SQLite table plan
- auth: existing RAG auth and workspace guard
- acceptance_ids: AC-RSG-08
- parameter_passing: existing request body or path/query parameters
- request: existing debug/retry DTOs
- response: existing uniform response envelope with graph source state or typed unsupported error
- client_impact: none required
- compatibility: additive

## Auth And Permissions

- Reuse existing RAG workspace access checks.
- Graph node visibility is limited to the same USER, AGENT, and SPACE scope semantics as RAG search.
- Related graph data returned in metadata must be reachable from a final visible hit and capped by profile policy.

## Error Contract

Format: `condition -> status/code; response; AC id`

- invalid related graph payload -> existing bad request mapping; uniform error response; AC-RSG-04
- disallowed node kind or relationship -> existing ingest validation error mapping; uniform error response; AC-RSG-04
- graph route unavailable with degrade allowed -> success with warning and base hits; AC-RSG-05
- graph route unavailable with fail-fast policy -> existing route unavailable error mapping; AC-RSG-05
- graph retry stage unsupported -> existing retry unsupported error mapping; uniform error response; AC-RSG-08

## Frontend Backend Boundary

No frontend changes are required. Existing clients can continue using the old `skill` corpus. Callers that opt into `skill-graph` select new corpus/profile ids and may read optional related graph metadata.

## Database And Transaction Boundary

Ingest persists one graph node and optional related graph objects through graph storage after RAG text and vector artifacts are prepared. SQLite remains the source for corpus/profile config and diagnostics. Cross-store atomicity is best-effort in the first version; failures are surfaced through existing ingest errors and diagnostics.

## Mock Or Adapter Expectations

Tests must use fake embedding and fake rerank runtimes. Graph storage tests use temporary local Grafeo storage. No test should require external model provider configuration or company network access.
