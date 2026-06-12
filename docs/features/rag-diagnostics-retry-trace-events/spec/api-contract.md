# API Contract: RAG 后端底座 - 诊断、重试、Trace 与事件

## API Overview

本阶段新增 RAG 诊断和索引重试 HTTP API，并扩展 search 请求的 trace 开关。所有 HTTP 响应继续使用统一响应体：

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {}
}
```

## Auth And Permissions

- 所有 debug/retry/search trace API 都要求已认证用户。
- `workspaceCode` 必须通过 Workspace Access Guard。
- Debug source 与 retry 必须限定 `scopeType` 和 `scopeCode`，不能只凭 sourceCode/sourceVersion 跨 scope 查询。
- RAG 仍不执行 corpus、source、repo、role、field 或内容脱敏权限；这些由调用方或上层模块负责。

## Endpoints

| method | path | purpose |
| --- | --- | --- |
| GET | `/api/v1/rag/debug/profile` | 只读查看 Profile 与 Corpus 配置摘要 |
| GET | `/api/v1/rag/debug/source` | 只读查看 Source、TextUnit 摘要和 warnings |
| POST | `/api/v1/rag/retry/index` | 对一个 Source 重试 fulltext/vector 索引阶段 |
| POST | `/api/v1/rag/search` | 增加可选 `trace.enabled`，启用轻量 search trace |

## GET /api/v1/rag/debug/profile

### Query

- `workspaceCode`: required string
- `profileId`: required string

### Success Data

```json
{
  "profile": {
    "profileId": "memory-related-v1",
    "profileType": "RETRIEVAL",
    "status": "enabled",
    "primaryCorpusId": "memory-card",
    "corpusIds": ["memory-card"],
    "requiredSourceVersion": "v1",
    "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE",
    "filters": {},
    "configSummary": {
      "routes": ["memory-fulltext", "memory-vector"],
      "contextStrategy": "MEMORY_CONTEXT_PACK"
    }
  },
  "corpora": [
    {
      "corpusId": "memory-card",
      "status": "enabled",
      "diagnosticsTable": "rag_diagnostics"
    }
  ]
}
```

## GET /api/v1/rag/debug/source

### Query

- `workspaceCode`: required string
- `corpusId`: required string, allowed `memory-card` or `project-doc`
- `sourceCode`: required string
- `sourceVersion`: required string
- `scopeType`: required string, allowed `USER`、`AGENT`、`SPACE`
- `scopeCode`: required string
- `includeTextUnits`: optional boolean, default `true`
- `includeWarnings`: optional boolean, default `true`

### Success Data

```json
{
  "source": {
    "corpusId": "project-doc",
    "sourceCode": "rag-mvp-doc",
    "sourceVersion": "v1",
    "scopeType": "SPACE",
    "scopeCode": "workspace_demo",
    "status": "ACTIVE",
    "title": "RAG MVP 设计方案",
    "metadata": {}
  },
  "textUnits": [
    {
      "textUnitCode": "tu_doc_001",
      "ordinal": 1,
      "status": "ACTIVE",
      "fulltextStatus": "INDEXED",
      "embeddingStatus": "INDEXED",
      "embeddingModelCode": "text-embedding-v1",
      "headingPath": "方案定位",
      "tokenCount": 120
    }
  ],
  "warnings": [
    {
      "recordCode": "diag_001",
      "stageId": "vectorIndex",
      "code": "RAG_VECTOR_INDEX_WARNING",
      "message": "vector index degraded",
      "createdAt": 1772294400000
    }
  ]
}
```

Debug source response must not include embedding BLOB, provider raw response, API key, or Context Pack body.

## POST /api/v1/rag/retry/index

### Request

```json
{
  "workspaceCode": "workspace_demo",
  "corpusId": "project-doc",
  "sourceCode": "rag-mvp-doc",
  "sourceVersion": "v1",
  "scopeType": "SPACE",
  "scopeCode": "workspace_demo",
  "stages": ["fulltextIndex", "vectorIndex"],
  "embeddingModelCode": "text-embedding-v1"
}
```

### Success Data

```json
{
  "corpusId": "project-doc",
  "sourceCode": "rag-mvp-doc",
  "sourceVersion": "v1",
  "status": "success",
  "stageResults": [
    {
      "stageId": "fulltextIndex",
      "status": "success",
      "updatedCount": 4,
      "warnings": []
    },
    {
      "stageId": "vectorIndex",
      "status": "success",
      "updatedCount": 4,
      "warnings": []
    }
  ],
  "warnings": [],
  "traceCode": "rag_retry_001"
}
```

### Partial Warning Data

```json
{
  "corpusId": "project-doc",
  "sourceCode": "rag-mvp-doc",
  "sourceVersion": "v1",
  "status": "partial_warning",
  "stageResults": [
    {
      "stageId": "vectorIndex",
      "status": "warning",
      "updatedCount": 0,
      "warnings": [
        {
          "code": "RAG_VECTOR_INDEX_WARNING",
          "message": "vector index degraded"
        }
      ]
    }
  ],
  "warnings": [
    {
      "code": "RAG_VECTOR_INDEX_WARNING",
      "stageId": "vectorIndex",
      "message": "vector index degraded"
    }
  ],
  "traceCode": "rag_retry_002"
}
```

Retry request must reject any stage except `fulltextIndex` and `vectorIndex`.

## POST /api/v1/rag/search Trace Addition

### Request Addition

```json
{
  "trace": {
    "enabled": true
  }
}
```

### Response Addition

```json
{
  "traceCode": "rag_trace_001"
}
```

Trace row stores query text and lightweight diagnostics only. It must not store hydrated body, Context Pack body, embedding, provider raw response or API key.

## EventBus Contract

RAG may publish these audit event names:

- `rag.ingest.started`
- `rag.ingest.completed`
- `rag.ingest.failed`
- `rag.search.started`
- `rag.search.completed`
- `rag.search.failed`
- `rag.retry.started`
- `rag.retry.completed`
- `rag.retry.failed`
- `rag.stage.completed`
- `rag.route.completed`

Payload allows codes, counts, status and warning codes only. Event publish failure is non-blocking for the main operation.

## Mock Or Adapter Expectations

- 后端测试使用 fake `LlmRuntimeService` runtime，不访问真实模型 provider。
- EventBus 测试使用 recording sink、disabled handle 或 failing publish adapter。
- SQLite 测试使用 in-memory database，执行现有 RAG migrations。
- Debug/retry HTTP 测试使用项目现有 auth test helper。

## Error Contract

| scenario | code |
| --- | --- |
| missing debug query | `RAG_DEBUG_REQUEST_INVALID` |
| profile not found | `RAG_PROFILE_NOT_FOUND` |
| source not found | `RAG_SOURCE_NOT_FOUND` |
| text units missing for retry | `RAG_RETRY_TEXT_UNITS_MISSING` |
| unsupported retry stage | `RAG_RETRY_STAGE_UNSUPPORTED` |
| retry request scope invalid | `RAG_RETRY_SCOPE_INVALID` |
| debug request scope invalid | `RAG_DEBUG_SCOPE_INVALID` |
| trace payload would contain forbidden field | `RAG_TRACE_PAYLOAD_INVALID` |
| event payload would contain forbidden field | `RAG_EVENT_PAYLOAD_INVALID` |

## Frontend Backend Boundary

- 本阶段没有前端页面。
- 前端或管理后台后续可以调用 debug/retry API，但不能拼接物理 SQL。
- Debug UI 不属于本阶段。

## Database And Transaction Boundary

- Debug API 只读，不开启写事务。
- Retry fulltext 和 vector 各自使用明确事务边界。
- Search trace 写入失败不应让 search 主流程失败；可返回 warning。
- EventBus 发布失败不回滚数据库事务。
