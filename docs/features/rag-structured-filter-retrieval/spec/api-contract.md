# API Contract: RAG Structured Filter Retrieval

## API Overview

The existing RAG search endpoint accepts filter-only structured retrieval when the selected Retrieval Profile contains a runnable `STRUCTURED_FILTER_SQLITE_V1` route. The endpoint and response envelope do not change.

## Endpoints

### POST /api/v1/rag/search

Request with omitted `query`:

```json
{
  "profileId": "memory-filter-v1",
  "accessContext": {
    "userCode": "USER_ADMIN",
    "agentCode": "AGENT_CODEX",
    "workspaceCode": "TRON_CORE"
  },
  "filters": {
    "sourceVersion": { "op": "eq", "value": "v1" },
    "memoryType": { "op": "eq", "value": "USER_NOTE" }
  },
  "limit": 20,
  "context": {
    "strategy": "RESULT_LIST"
  }
}
```

Success response:

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {
    "profileId": "memory-filter-v1",
    "query": "",
    "contextStrategy": "RESULT_LIST",
    "hits": [],
    "contextPack": null,
    "warnings": [],
    "traceCode": null
  }
}
```

## Auth And Permissions

- HTTP route remains authenticated.
- Request `accessContext.userCode` must match authenticated user.
- User must have workspace access to `accessContext.workspaceCode`.
- Search scopes remain direct USER, AGENT, and SPACE scopes.
- No new corpus, source, role, repo, or field-level permission model is added.

## Error Contract

| code | condition | http status |
| --- | --- | --- |
| `RAG_SEARCH_REQUEST_INVALID` | blank query with no executable structured-filter route, missing required non-query fields, invalid context strategy, or invalid limit | 400 |
| `RAG_SEARCH_FILTER_INVALID` | filter field undeclared, type invalid, operator invalid, or request filter broadens profile constraints | 400 |
| `RAG_RETRIEVAL_PROFILE_NOT_FOUND` | profile does not exist | 404 |
| `RAG_RETRIEVAL_PROFILE_INVALID` | profile corpus, route, strategy, version, or context configuration invalid | 400 |
| `RAG_SEARCH_SCOPE_INVALID` | access context does not match authenticated user or workspace access | 403 |
| `RAG_SEARCH_ROUTE_UNAVAILABLE` | runnable route is unavailable under fail-fast behavior | 503 |

## Frontend Backend Boundary

- No frontend page or profile editor is added.
- Future frontend configuration may expose the new retriever ID, but this slice is backend-only.
- API clients should treat blank query as valid only when the selected profile is designed for structured retrieval.

## Database And Transaction Boundary

- Search remains read-only.
- No migrations, writes, or trace rows are added.
- Existing SQLite transaction behavior is unchanged.

## Mock Or Adapter Expectations

- Tests use existing fake runtime behavior and must not call real model providers.
- SQLite adapter implements the first structured-filter retriever.
- GraphBacked adapter remains out of scope for structured-filter search.
