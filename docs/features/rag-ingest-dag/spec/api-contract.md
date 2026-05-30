# API Contract: RAG 后端底座 - 同步写入 DAG

## API Overview

本 feature 新增 RAG ingest HTTP API 和内部 Rust service contract。API 用于 Memory 和 Project Doc 调用方提交文本内容，RAG 按同步 DAG 写入 Source、TextUnit、全文字段、向量字段和 warnings。

响应必须使用项目统一响应体：

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {}
}
```

## Endpoints

### POST /api/v1/rag/ingest

Request:

```json
{
  "corpusId": "project-doc",
  "ingestProfileId": "project-doc-ingest-v1",
  "workspaceCode": "TRON_CORE",
  "accessContext": {
    "userCode": "USER_ADMIN",
    "agentCode": "AGENT_CODEX",
    "workspaceCode": "TRON_CORE"
  },
  "targetScope": {
    "scopeType": "SPACE",
    "scopeCode": "TRON_CORE"
  },
  "sourceCode": "doc-rag-mvp",
  "sourceVersion": "v1",
  "sourceType": "MARKDOWN",
  "payload": {
    "title": "RAG MVP 设计方案",
    "text": "# RAG MVP\\n\\n## 写入\\n同步 DAG。",
    "documentCode": "DOC_RAG_MVP",
    "documentType": "DESIGN",
    "path": "docs/rag-mvp.md",
    "repoCode": "REPO_TRON",
    "moduleCode": "MOD_RAG",
    "moduleName": "RAG"
  }
}
```

Success response:

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {
    "corpusId": "project-doc",
    "profileId": "project-doc-ingest-v1",
    "sourceCode": "doc-rag-mvp",
    "sourceVersion": "v1",
    "status": "ACTIVE",
    "textUnitCount": 2,
    "fulltextStatus": "INDEXED",
    "embeddingStatus": "INDEXED",
    "warnings": []
  }
}
```

Project Doc vector degrade response:

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {
    "corpusId": "project-doc",
    "profileId": "project-doc-ingest-v1",
    "sourceCode": "doc-rag-mvp",
    "sourceVersion": "v1",
    "status": "ACTIVE",
    "textUnitCount": 2,
    "fulltextStatus": "INDEXED",
    "embeddingStatus": "FAILED",
    "warnings": [
      {
        "stageId": "vectorIndex",
        "code": "RAG_VECTOR_INDEX_WARNING",
        "message": "Project Doc vector indexing failed; fulltext remains available"
      }
    ]
  }
}
```

### Internal Rust Contract

```rust
pub async fn ingest(&self, request: RagIngestRequest) -> Result<RagIngestResult, RagError>;
```

The service owns request validation, profile loading, DAG compilation, stage execution, persistence, warning persistence, and final Source status update.

## Auth And Permissions

- HTTP route must require authenticated current user.
- `workspaceCode` must pass existing workspace access guard.
- `targetScope.scopeType=USER` can only write `accessContext.userCode`。
- `targetScope.scopeType=AGENT` can only write `accessContext.agentCode`。
- `targetScope.scopeType=SPACE` can only write `accessContext.workspaceCode`。
- Write path does not apply workspace inheritance.
- RAG does not perform corpus, source, repo, role, field, or content permission checks.

## Error Contract

| code | condition | http status |
| --- | --- | --- |
| `RAG_INGEST_REQUEST_INVALID` | required field missing or payload shape invalid | 400 |
| `RAG_INGEST_PROFILE_NOT_FOUND` | profile does not exist | 404 |
| `RAG_INGEST_PROFILE_INVALID` | profile corpus, DAG, or strategy references are invalid | 400 |
| `RAG_INGEST_SCOPE_INVALID` | target scope does not match access context | 403 |
| `RAG_INGEST_STAGE_FAILED` | required stage fails | 500 |
| `RAG_EMBEDDING_FAILED` | memory-card vector indexing fails | 502 |
| `RAG_CORPUS_RESERVED` | caller writes reserved corpus such as code-graph | 400 |

All errors must be wrapped in the uniform response body.

## Frontend Backend Boundary

- No frontend page is added in this feature.
- API is consumed by backend modules and future agent/runtime integrations.
- UI copy, browser flows, and Profile management UI are out of scope.

## Database And Transaction Boundary

- Persist stage runs in one repository transaction for Source and TextUnit upserts.
- Fulltext field update is part of TextUnit persistence.
- Vector field update runs after embedding succeeds.
- Source status update happens after required stages finish.
- Project Doc vector degrade commits fulltext/TextUnit data and persists warning.
- Memory vector failure marks Source failed and returns an error.

## Mock Or Adapter Expectations

- Tests use a fake embedding runtime implementing the LLM application port.
- Tests must not call real model providers.
- Tests must not use GraphService.
- Tests may call HTTP route through app router with authenticated fixtures.
- Repository tests should use SQLite temporary files or in-memory databases as appropriate.
