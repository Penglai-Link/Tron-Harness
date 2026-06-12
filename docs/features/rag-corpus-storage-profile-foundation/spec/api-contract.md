# API Contract: RAG 后端底座 - Corpus 存储与 Profile 基础

## API Overview

本 feature 定义 RAG foundation 的内部 Rust API 契约，不新增 HTTP endpoint、route、controller 或 frontend 页面。当前阶段只提供 Corpus、Profile、storage binding、filterSchema 和 strategy registry 的只读查询与校验边界。

后续 RAG Debug HTTP API 可以复用本契约暴露只读诊断能力，但 debug route、trace 展示、retry 操作、ingest 和 search 都不属于本 feature。

所有未来 HTTP 响应必须继续使用项目统一 envelope：

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {}
}
```

内部 Rust contract 的建议模块：

- `tron-backend/src/domain/rag.rs`
- `tron-backend/src/domain/rag/*`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/*`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`

## Endpoints

- none
- 本 feature 不新增 `GET`、`POST`、`PUT`、`PATCH` 或 `DELETE` route。
- 不新增 `/api/v1/rag/ingest`。
- 不新增 `/api/v1/rag/search`。
- 不新增 `/api/v1/rag/debug/*`。
- 现有 frontend endpoint 字符串不需要变更。

### Internal Rust Contract

`RagService` 提供只读配置查询和校验方法：

```rust
pub struct RagService<R, S> {
    repository: R,
    strategy_registry: S,
}

impl<R, S> RagService<R, S>
where
    R: RagConfigRepository,
    S: RagStrategyRegistry,
{
    pub fn list_corpora(&self) -> Result<Vec<RagCorpusView>, RagError>;
    pub fn get_corpus(&self, corpus_id: &str) -> Result<RagCorpusView, RagError>;
    pub fn get_storage_binding(&self, corpus_id: &str) -> Result<RagStorageBindingView, RagError>;
    pub fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileView>, RagError>;
    pub fn get_profile(&self, profile_id: &str) -> Result<RagProfileView, RagError>;
    pub fn validate_profile(&self, profile_id: &str) -> Result<RagProfileValidationView, RagError>;
    pub fn validate_filters(&self, request: RagFilterValidationRequest) -> Result<RagFilterValidationView, RagError>;
}
```

`RagConfigRepository` 是 persistence port，不暴露 SQLite 细节：

```rust
pub trait RagConfigRepository: Clone + Send + Sync + 'static {
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagRepositoryError>;
    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagRepositoryError>;
    fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileRecord>, RagRepositoryError>;
    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagRepositoryError>;
}
```

`RagStrategyRegistry` 是代码内有限注册表：

```rust
pub trait RagStrategyRegistry: Clone + Send + Sync + 'static {
    fn resolve_stage(&self, strategy_id: &str) -> StrategyRegistration;
    fn resolve_retriever(&self, retriever_id: &str) -> StrategyRegistration;
    fn resolve_policy(&self, policy_id: &str) -> StrategyRegistration;
}
```

`StrategyRegistration.status` 只允许：

- `enabled`
- `reserved`
- `unsupported`

### DTO Shape

对外 service DTO 使用 camelCase 字段，数据库字段仍使用 snake_case。

`RagCorpusView`：

```json
{
  "corpusId": "memory-card",
  "displayName": "长期记忆",
  "status": "enabled",
  "description": "",
  "capabilities": {},
  "filterSchema": {},
  "fieldMapping": {},
  "storageBinding": {
    "sourceTable": "rag_memory_sources",
    "textUnitTable": "rag_memory_text_units",
    "diagnosticsTable": "rag_diagnostics"
  }
}
```

`RagProfileView`：

```json
{
  "profileId": "memory-related-v1",
  "profileType": "RETRIEVAL",
  "status": "enabled",
  "primaryCorpusId": "memory-card",
  "corpusIds": ["memory-card"],
  "requiredSourceVersion": "v1",
  "config": {},
  "filters": {},
  "degradePolicy": "SKIP_VECTOR_ON_UNAVAILABLE"
}
```

`RagProfileListQuery`：

```json
{
  "profileType": "RETRIEVAL",
  "status": "enabled",
  "corpusId": "memory-card"
}
```

所有字段都是可选过滤条件。`corpusId` 匹配 `primaryCorpusId` 或 `corpusIds` 中的任意一项。

`RagProfileValidationView`：

```json
{
  "profileId": "memory-related-v1",
  "valid": true,
  "status": "enabled",
  "errors": [],
  "strategyRefs": [
    {
      "kind": "retriever",
      "strategyId": "FULLTEXT",
      "status": "enabled"
    }
  ]
}
```

`RagFilterValidationRequest`：

```json
{
  "corpusId": "memory-card",
  "filters": {
    "scopeType": "SPACE",
    "memoryType": ["RULE", "DECISION"]
  }
}
```

`RagFilterValidationView`：

```json
{
  "valid": true,
  "normalizedFilters": {},
  "fieldErrors": []
}
```

## Auth And Permissions

- 当前 feature 不新增 HTTP route，因此不新增 HTTP authentication 或 permission check。
- 后续 debug HTTP route 必须挂在现有 protected routes 下，并复用 `ApiResponse<T>`。
- 本 feature 的 filterSchema 校验不是权限系统；它只判断字段是否声明、类型是否匹配、操作符是否允许。
- 后续 ingest/search 调用方仍必须传递 `userCode`、`agentCode`、`workspaceCode` 或等价 AccessContext；当前 feature 只保留 code 语义和 scope 字段边界。
- RAG foundation 不接管 corpus 权限、source 权限、repo 权限、角色权限、字段权限或内容脱敏。
- 只读查询可以返回 disabled/reserved Corpus 或 Profile；只有后续执行型调用才必须拒绝 disabled/reserved 配置。

## Error Contract

内部 `RagError` 应映射为稳定错误码。未来 HTTP route 使用 `ApiResponse::error(code, message)` 返回错误，`data` 为空对象。

| code | condition | future http status |
| --- | --- | --- |
| `RAG_CORPUS_NOT_FOUND` | `corpusId` 不存在 | 404 |
| `RAG_CORPUS_RESERVED` | 执行型调用请求 reserved Corpus | 409 |
| `RAG_PROFILE_NOT_FOUND` | `profileId` 不存在 | 404 |
| `RAG_PROFILE_DISABLED` | Profile 状态为 disabled | 409 |
| `RAG_PROFILE_RESERVED` | 执行型调用请求 reserved Profile | 409 |
| `RAG_PROFILE_SOURCE_VERSION_REQUIRED` | Retrieval Profile 缺少固定 `requiredSourceVersion` | 400 |
| `RAG_STRATEGY_UNREGISTERED` | Profile 引用未注册 strategy ID | 400 |
| `RAG_STRATEGY_RESERVED` | enabled Profile 引用 reserved strategy ID | 400 |
| `RAG_FILTER_FIELD_UNKNOWN` | filter 字段未在 filterSchema 声明 | 400 |
| `RAG_FILTER_TYPE_INVALID` | filter 值类型与 filterSchema 不匹配 | 400 |
| `RAG_FILTER_OPERATOR_UNSUPPORTED` | filter 操作符不在字段允许范围内 | 400 |
| `RAG_STORAGE_BINDING_INVALID` | Corpus storage binding 缺少必要表绑定或表名非法 | 500 |
| `RAG_STORAGE_FAILED` | SQLite repository 读取失败 | 500 |

错误消息使用中文描述原因，错误码保持英文稳定常量。

## Frontend Backend Boundary

- 当前 feature 无 frontend 改动。
- Frontend 不应依赖本 feature 新增任何 route。
- 后续 RAG Debug 页面只能读取配置、binding、校验结果和诊断摘要，不能修改 Profile、Corpus 或 storage binding。
- 后续 RAG Debug 页面不能展示 `embedding` BLOB、provider 原始响应、API key、hydrate 正文或 Context Pack body。
- `storageBinding` 可以作为 debug 信息展示，但不能作为 frontend 选择物理表或拼接查询的输入。

## Database And Transaction Boundary

- `SqliteRagRepository::connect(database_url)` 负责执行 RAG foundation migration 和 seed。
- 当前 service 方法只读 `rag_corpora` 与 `rag_profiles`，解析其中 JSON 字段后在 application 层校验。
- 当前 service 不写 `rag_memory_sources`、`rag_project_doc_sources`、`rag_memory_text_units`、`rag_project_doc_text_units` 或 `rag_diagnostics`。
- 当前 service 不开启业务写事务；migration/seed 使用 repository 初始化边界完成。
- Profile 校验需要读取 Corpus 配置并访问 `RagStrategyRegistry`，但不能调用 `LlmRuntimeService`、`GraphService` 或 `EventBusHandle`。
- `requiredSourceVersion` 必须是具体版本值；不支持 `latest`，写入数据也不维护 latest 标记。

## Mock Or Adapter Expectations

- Unit tests 可以使用 SQLite in-memory repository 执行 migration 和 seed。
- Unit tests 可以使用 fake `RagStrategyRegistry` 覆盖 enabled、reserved、unsupported 三类 strategy。
- 不需要 HTTP mock adapter。
- 不需要 frontend mock data。
- 不需要模型 provider mock，因为当前 feature 不生成 embedding，也不执行 rerank。
- 不需要 GraphService fake，因为当前 feature 不接入 graph。
