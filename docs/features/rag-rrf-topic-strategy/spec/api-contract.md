# API Contract: RAG Multi-Corpus RRF And Topic Strategy

## API Overview

No new endpoint is added. Existing RAG search requests continue to use configured retrieval profiles; new policies are profile configuration internals.

## Endpoints

- `POST /api/v1/rag/search`: unchanged request path and response envelope.

## Auth And Permissions

Existing authenticated workspace access rules continue to apply.

## Error Contract

- Invalid profile policy config maps to existing retrieval profile validation errors.
- Missing vector candidates for the topic policy do not create a new error.
- Existing uniform response body remains unchanged.

## Frontend Backend Boundary

No frontend changes are required for this backend-only slice.

## Database And Transaction Boundary

Search remains read-only except existing optional diagnostics trace writes.

## Mock Or Adapter Expectations

- SQLite search adapter returns optional candidate embeddings when persisted.
- Fake search runtime in tests controls query and ingest embeddings deterministically.
