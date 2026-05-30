# API Contract: RAG 后端底座 - Memory 与 Project Doc 领域字段

## API Overview

本 feature 定义 RAG 领域字段的内部 Rust API 契约，不新增 HTTP endpoint、route、controller 或 frontend 页面。当前阶段只扩展 Corpus 配置、字段映射和过滤校验能力。

未来 ingest API、search API、debug API 的 request/response 可以使用这些 logical fields，但本 feature 不实现这些 HTTP 行为。

## Endpoints

- none
- 不新增 `/api/v1/rag/ingest`。
- 不新增 `/api/v1/rag/search`。
- 不新增 `/api/v1/rag/debug/*`。
- 不新增 frontend endpoint 字符串。

### Internal Rust Contract

沿用上一阶段 `RagService`：

```rust
pub fn get_corpus(&self, corpus_id: &str) -> Result<RagCorpusView, RagError>;
pub fn validate_filters(&self, request: RagFilterValidationRequest) -> Result<RagFilterValidationView, RagError>;
```

本阶段 contract 的变化是 `RagCorpusView.filter_schema` 和 `RagCorpusView.field_mapping` 增加 Memory 与 Project Doc logical fields。

### Memory Filter Example

```json
{
  "corpusId": "memory-card",
  "filters": {
    "memoryType": ["RULE", "DECISION"],
    "reviewStatus": "APPROVED",
    "injectable": true,
    "pinLevel": { "op": "gte", "value": 1 }
  }
}
```

期望 normalized filters：

```json
{
  "memoryType": { "op": "in", "value": ["RULE", "DECISION"] },
  "reviewStatus": { "op": "eq", "value": "APPROVED" },
  "injectable": { "op": "eq", "value": true },
  "pinLevel": { "op": "gte", "value": 1 }
}
```

### Project Doc Filter Example

```json
{
  "corpusId": "project-doc",
  "filters": {
    "repoCode": "REPO-TRON",
    "moduleCode": "MOD-RAG",
    "documentType": ["ADR", "DESIGN"],
    "path": "docs/rag.md"
  }
}
```

## Auth And Permissions

- 当前 feature 不新增 HTTP route，因此不新增 HTTP authentication 或 permission check。
- RAG 字段校验不是权限系统。
- `repoCode`、`moduleCode`、`memoryStatus` 等字段只参与存储和过滤，不表达访问权限。
- RAG 不执行 corpus、source、repo、role、field 或内容脱敏权限。
- 后续 HTTP ingest/search 仍必须先通过 workspace access guard，再进入 RAG 应用服务。

## Error Contract

沿用上一阶段内部错误码语义。新增字段相关错误仍使用 filterSchema 错误：

| code | condition | future http status |
| --- | --- | --- |
| `RAG_FILTER_FIELD_UNKNOWN` | filter 字段未在当前 Corpus filterSchema 声明 | 400 |
| `RAG_FILTER_TYPE_INVALID` | filter 值类型与 filterSchema 不匹配 | 400 |
| `RAG_FILTER_OPERATOR_UNSUPPORTED` | filter 操作符不在字段允许范围内 | 400 |
| `RAG_STORAGE_BINDING_INVALID` | Corpus binding 或 filterSchema 配置非法 | 500 |
| `RAG_STORAGE_FAILED` | SQLite repository 读取或 migration 失败 | 500 |

错误消息保持中文，错误码保持英文稳定常量。

## Frontend Backend Boundary

- 当前 feature 无 frontend 改动。
- Frontend 不应依赖本 feature 新增 route。
- 后续 debug 页面可以展示字段映射和 filterSchema，但不能让 frontend 拼接物理 SQL。
- `storageBinding` 和 physical column 只用于 debug 可见性，不作为前端查询输入。

## Database And Transaction Boundary

- `SqliteRagRepository::connect(database_url)` 负责执行 foundation migration 和领域字段 schema evolution。
- 新增列通过 column existence check 保护，避免重复初始化失败。
- 当前 service 方法只读 `rag_corpora` 和解析 JSON 配置。
- 当前 feature 不写 Source/TextUnit 业务行，不开启 ingest/search 事务。
- 当前 feature 不调用模型、Graph 或 EventBus。

## Mock Or Adapter Expectations

- Unit tests 使用 SQLite `:memory:` repository。
- 不需要 HTTP mock。
- 不需要 frontend mock data。
- 不需要模型 provider mock。
- 不需要 GraphService fake。
- 可以使用现有 `RagService::validate_filters` 直接覆盖新增 logical fields。
