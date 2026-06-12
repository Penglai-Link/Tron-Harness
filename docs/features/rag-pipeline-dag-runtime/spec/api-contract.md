# API Contract: RAG Pipeline DAG Runtime

## API Overview

The feature adds a Pipeline Catalog API and upgrades Profile management payloads. Runtime ingest and search request bodies remain unchanged. Runtime responses are backward compatible and may include additive `stageResults`.

## Endpoints

- `GET /api/v1/rag/pipeline/catalog?profileType=INGEST`
- `GET /api/v1/rag/pipeline/catalog?profileType=RETRIEVAL`
- `POST /api/v1/rag/profiles`
- `POST /api/v1/rag/profiles/update`
- `POST /api/v1/rag/profiles/verify`
- `POST /api/v1/rag/ingest`
- `POST /api/v1/rag/search`

## Auth And Permissions

Management APIs keep the existing authenticated management behavior. Runtime ingest/search keep existing request auth and workspace access checks.

## Error Contract

All HTTP responses use the project uniform body:

```json
{
  "code": "0",
  "message": "success",
  "data": {}
}
```

Pipeline validation failures return existing RAG management validation error semantics with field-level issue rows where applicable.

## Frontend Backend Boundary

Frontend must fetch catalog data and render stage/strategy options from backend or mock catalog. Frontend must send the new `pipeline` payload for profile saves and verification.

## Database And Transaction Boundary

Profile create/update persists one profile row through the existing repository save path. Runtime ingest/search persistence remains owned by existing RAG runtime repositories.

## Mock Or Adapter Expectations

Mock API must expose Pipeline Catalog, Profile DAG payloads, and runtime `stageResults` for browser tests. Adapter code must tolerate absent `stageResults` to preserve old runtime responses.
