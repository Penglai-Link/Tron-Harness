# API Contract: RAG 管理配置 API 接入

## API Overview

All management APIs stay under `/api/v1/rag` and use the existing response envelope:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {}
}
```

Errors keep the same `{ code, message, data }` shape with RAG-specific error codes.

## Endpoints

### GET /api/v1/rag/corpora

Returns corpus list cards for the RAG management overview.

`data[]` fields:

- `corpusId`
- `displayName`
- `description`
- `status`
- `capabilities`
- `metadataGroups`
- `storageSummary`
- `ingestProfileId`
- `retrievalProfileId`

### GET /api/v1/rag/corpora/{corpusId}

Returns a corpus form DTO.

`data` fields:

- `corpusId`
- `displayName`
- `description`
- `status`
- `capabilities`
- `metadataGroups`
- `filterSchema`
- `fieldMapping`
  - `source`
  - `textUnit`
- `storageBinding`
  - `mode: TARGETED`
  - `targets[]`
  - `bindings{}`
    - `source`
    - `textUnit`
    - `fulltext`
    - `vector`
    - `diagnostics` remains optional/derived and is not user-edited by the corpus form
- `runtimeProjection`
  - `compatible`
  - `legacyBinding`
    - optional `fulltextTable` when SQLite fulltext storage is bound independently from `textUnitTable`
    - optional `vectorTable` when SQLite vector storage is bound independently from `textUnitTable`
    - optional graph fulltext/vector namespace and node label fields when retrieval carriers are Graph-backed
  - `issues[]`

### GET /api/v1/rag/corpora/catalog

Returns form support data for corpus create/edit.

`data` fields:

- `storageBackends`
- `codeStrategies`
- `contractFields`
  - `source[]`
  - `textUnit[]`
- `filterFieldTypes`
- `filterOperators`
- `capabilityRules`
- `compatibilityRules`

### POST /api/v1/rag/corpora/verify

Request body:

- `corpusId`
- `displayName`
- `description`
- `status`
- `capabilities`
- `metadataGroups`
- `filterSchema`
- `fieldMapping`
- `storageBinding`

Response `data` fields:

- `valid`
- `errors[]`
- `runtimeProjection`

### POST /api/v1/rag/corpora

Creates a corpus from the same form DTO used by verify.

### PUT /api/v1/rag/corpora/{corpusId}

Updates a corpus from the same form DTO used by verify.

### GET /api/v1/rag/profiles

Query params:

- `profileType=INGEST|RETRIEVAL` optional

Returns profile list rows.

### GET /api/v1/rag/profiles/{profileId}

Returns a profile form DTO.

Common fields:

- `profileId`
- `profileType`
- `status`
- `description`
- `primaryCorpusId`
- `corpusIds`
- `requiredSourceVersion`

Ingest-only fields:

- `pipelineStages[]`
- `embeddingModelCode`
- `embeddingVersion`
- `graphWritePolicy`
- `sourceInputPolicy`

Retrieval-only fields:

- `routes[]`
- `filters[]`
- `fusionPolicy`
- `dedupPolicy`
- `topicPolicy`
- `graphExpansionPolicy`
- `contextStrategies[]`
- `degradePolicy`

### GET /api/v1/rag/profiles/catalog

Query params:

- `profileType=INGEST|RETRIEVAL` optional

Response `data` fields:

- `corpora[]`
- `ingestStageDefinitions[]`
- `ingestStageStrategies`
- `retrievalRouteTypes[]`
- `retrievalStrategies`
- `fusionPolicyTypes[]`
- `dedupPolicyTypes[]`
- `topicPolicyTypes[]`
- `graphExpansionPolicies[]`
- `contextStrategies[]`

### POST /api/v1/rag/profiles/verify

Request body uses the same profile form DTO returned by detail.

Response `data` fields:

- `valid`
- `errors[]`

### POST /api/v1/rag/profiles

Creates a profile from the same form DTO used by verify.

### PUT /api/v1/rag/profiles/{profileId}

Updates a profile from the same form DTO used by verify.

### POST /api/v1/rag/filters/validate

Request body:

- `corpusId`
- `filters`

Response `data` fields:

- `valid`
- `normalizedFilters`
- `fieldErrors[]`

## Error Codes

- `RAG_CORPUS_NOT_FOUND`
- `RAG_PROFILE_NOT_FOUND`
- `RAG_MANAGEMENT_REQUEST_INVALID`
- `RAG_STORAGE_TOPOLOGY_UNSUPPORTED`
- `RAG_STORAGE_GRAPH_BINDING_REQUIRED`
- `RAG_FILTER_FIELD_INVALID`
- `RAG_FILTER_OPERATOR_UNSUPPORTED`
- `RAG_PROFILE_STRATEGY_INVALID`
- `RAG_PROFILE_CORPUS_INVALID`

## Auth And Permissions

- Management APIs are bearer-protected through the existing protected router wiring.
- This phase does not introduce extra per-route RBAC checks beyond the current authenticated management surface.
- If a future evolution makes RAG management workspace-scoped, reuse the existing workspace-membership guard pattern already used by runtime RAG handlers.

## Error Contract

- Validation and compatibility failures must return stable RAG-specific error codes plus human-readable messages.
- List/detail not-found cases return not-found style RAG errors rather than generic ingest/search runtime errors.
- Verify endpoints return field or rule violations without mutating persistence state.

## Frontend Backend Boundary

- Frontend pages consume form-friendly HTTP DTOs and must not reconstruct raw backend persistence records on the client.
- Management `storageBinding` remains the source of truth for editable topology, while `runtimeProjection.legacyBinding` is compatibility-oriented and may use fallback semantics for current runtime execution.
- Management APIs normalize omitted `bindings.fulltext` to the selected `textUnit` target for legacy clients, but new clients must submit `source`, `textUnit`, `fulltext`, and `vector` explicitly.
- `capabilities.graph=true` is valid only when at least one non-diagnostics segment binding resolves to a `GRAPH` target. It no longer requires all master data segments to be Graph-backed.
- Backend catalog endpoints own dropdown and rule metadata that is currently mocked in the RAG pages.
- Frontend browser tests may mock endpoint payloads to prove page behavior, but production API mode must use the real endpoint definitions and request wrapper.

## Database And Transaction Boundary

- Verify endpoints are read-only and must not persist mutations.
- Create/update endpoints own topology validation, runtime projection, and persistence of both management and legacy compatibility fields in one save path.
- Runtime projection preserves role-aware carrier targets in `storage_binding_json` while still retaining legacy fields for old readers.
- Profile create/update should normalize into the existing runtime-readable profile storage fields without forcing runtime execution changes in the same transaction.

## Mock Or Adapter Expectations

- Existing frontend mock registries remain only as development fallback or migration aid until the relevant page path is switched to API mode.
- Management HTTP handlers should follow the request/response DTO and error-mapper patterns from other CRUD modules rather than reusing runtime RAG request DTOs.
- Backend catalog responses may be backed by static registry/config data in phase 1, but the HTTP contract should already match the page’s real selection needs.

## Compatibility Notes

- Runtime routes `POST /api/v1/rag/ingest`, `POST /api/v1/rag/search`, `GET /api/v1/rag/debug/profile`, `GET /api/v1/rag/debug/source`, and `POST /api/v1/rag/retry/index` remain unchanged.
- Corpus detail and save APIs may expose both management topology and projected legacy binding, but runtime readers still consume the projected legacy binding.
- Legacy corpus rows without `fulltext` bindings or `fulltextTable` keep their previous behavior by deriving fulltext storage from `textUnitTable`.
