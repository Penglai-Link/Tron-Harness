# API Contract: RAG Skill Graph Policy

## API Overview

This feature uses existing RAG HTTP APIs. It does not add endpoints or change the uniform response envelope.

## Endpoints

### Existing RAG Ingest API

- purpose: callers can pass `payload.relatedGraph.nodes` and `payload.relatedGraph.edges` when using `skill-graph`.
- request: existing ingest DTO; `edges[].properties` may contain scalar custom values.
- response: existing ingest result envelope.
- compatibility: old callers and old corpora remain unchanged.

### Existing RAG Search API

- purpose: callers can request graph relation expansion through existing `context.graphExpansion.relationships`.
- request: relationship filter remains request-driven; missing relationships query all edge relationships within configured depth.
- response: existing result hits; graph hits may include `metadata.relatedGraph.nodes` and `metadata.relatedGraph.edges[].properties`.
- compatibility: top-level schema unchanged.

## Auth And Permissions

Existing RAG auth, workspace access, and graph scope semantics apply.

## Error Contract

- invalid node/edge custom property -> existing ingest invalid request error.
- configured whitelist miss -> existing ingest invalid request error.
- invalid graph expansion relationship request code -> existing search invalid request error.

## Frontend Backend Boundary

No frontend changes are required.

## Database And Transaction Boundary

Profile configuration is read from SQLite. Graph writes are performed through the existing graph store after ingest artifacts are prepared.

## Mock Or Adapter Expectations

Tests use fake embedding/rerank runtimes and temporary local graph storage.
