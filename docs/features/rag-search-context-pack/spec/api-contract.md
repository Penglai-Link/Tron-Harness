# API Contract: RAG 后端底座 - 检索编排与 Context Pack

## API Overview

本 feature 新增 RAG search HTTP API 和内部 Rust service contract。API 用于 Memory、Project Doc 和 Agent 运行时按 Retrieval Profile 检索知识，并返回结果列表或 Context Pack。

响应必须使用项目统一响应体：

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {}
}
```

## Endpoints

### POST /api/v1/rag/search

Request:

```json
{
  "profileId": "project-doc-related-v1",
  "workspaceCode": "TRON_CORE",
  "query": "RAG 写入流程如何执行",
  "accessContext": {
    "userCode": "USER_ADMIN",
    "agentCode": "AGENT_CODEX",
    "workspaceCode": "TRON_CORE"
  },
  "filters": {
    "sourceVersion": "v1",
    "documentType": ["DESIGN"],
    "repoCode": "REPO_TRON"
  },
  "limit": 5,
  "tokenBudget": 1200,
  "context": {
    "strategy": "DOC_QA_CONTEXT_PACK"
  }
}
```

Success response:

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {
    "profileId": "project-doc-related-v1",
    "query": "RAG 写入流程如何执行",
    "contextStrategy": "DOC_QA_CONTEXT_PACK",
    "hits": [
      {
        "corpusId": "project-doc",
        "sourceCode": "doc-rag-mvp",
        "sourceVersion": "v1",
        "targetType": "TEXT_UNIT",
        "targetCode": "doc-rag-mvp:v1:1",
        "routeIds": ["project-doc-fulltext", "project-doc-vector"],
        "score": 1.23,
        "scopeType": "SPACE",
        "scopeCode": "TRON_CORE",
        "preview": "写入使用同步 DAG...",
        "text": "写入使用同步 DAG...",
        "metadata": {
          "title": "RAG MVP 设计方案",
          "path": "docs/rag-mvp.md",
          "headingPath": ["写入"]
        }
      }
    ],
    "contextPack": {
      "items": [
        {
          "sourceCode": "doc-rag-mvp",
          "text": "写入使用同步 DAG...",
          "tokens": 12
        }
      ],
      "tokenCount": 12
    },
    "warnings": []
  }
}
```

Vector route degrade response:

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {
    "profileId": "project-doc-related-v1",
    "query": "RAG 写入流程如何执行",
    "contextStrategy": "RESULT_LIST",
    "hits": [],
    "contextPack": null,
    "warnings": [
      {
        "routeId": "project-doc-vector",
        "code": "RAG_VECTOR_ROUTE_UNAVAILABLE",
        "message": "Vector route skipped; fulltext route remains available"
      }
    ]
  }
}
```

### Internal Rust Contract

```rust
pub async fn search(&self, request: RagSearchRequest) -> Result<RagSearchResult, RagError>;
```

The service owns request validation, Retrieval Profile loading, filter merging, route execution, fusion, dedup, optional rerank, hydrate, and context assembly.

## Auth And Permissions

- HTTP route must require authenticated current user.
- `workspaceCode` must pass existing workspace access guard.
- Request `accessContext.userCode` must match authenticated user code.
- Search scope includes current `USER:userCode`、`AGENT:agentCode`、`SPACE:workspaceCode`。
- 第一版不解析 workspace ancestors。
- RAG does not perform corpus, source, repo, role, field, or content permission checks.

## Error Contract

| code | condition | http status |
| --- | --- | --- |
| `RAG_SEARCH_REQUEST_INVALID` | required field missing, query blank, limit invalid, or payload shape invalid | 400 |
| `RAG_RETRIEVAL_PROFILE_NOT_FOUND` | profile does not exist | 404 |
| `RAG_RETRIEVAL_PROFILE_INVALID` | profile corpus, routes, strategy, context, or version config invalid | 400 |
| `RAG_SEARCH_FILTER_INVALID` | filter field is undeclared or value type invalid | 400 |
| `RAG_SEARCH_VERSION_REQUIRED` | request and profile both omit fixed sourceVersion | 400 |
| `RAG_SEARCH_SCOPE_INVALID` | access context does not match authenticated request | 403 |
| `RAG_SEARCH_ROUTE_UNAVAILABLE` | all enabled routes are unavailable | 503 |
| `RAG_HYDRATE_SOURCE_INACTIVE` | hydrate finds source not ACTIVE | 409 |
| `RAG_RERANK_FAILED` | rerank fails and profile is fail-fast | 502 |

All errors must be wrapped in the uniform response body.

## Frontend Backend Boundary

- No frontend page is added in this feature.
- API is consumed by backend modules and future Agent/runtime integrations.
- UI copy, browser flows, Profile management UI, debug UI, retry UI and trace UI are out of scope.

## Database And Transaction Boundary

- Search is read-only for RAG Source/TextUnit/Profile data.
- Search does not write trace rows in this feature.
- Vector route may call model runtime but does not persist query embedding.
- Hydrate must re-check Source status after route recall.
- Source/TextUnit writes remain owned by ingest.

## Mock Or Adapter Expectations

- Tests use fake embedding and fake rerank runtimes.
- Tests must not call real model providers.
- Tests must not use GraphService.
- Tests may seed data through RAG ingest service, then call search service or HTTP route.
- Repository tests should use SQLite temporary files or in-memory databases as appropriate.
