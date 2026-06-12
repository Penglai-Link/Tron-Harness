# Database Design: RAG 后端底座 - 诊断、重试、Trace 与事件

## Database Decision

- status: approved
- migration_required: no_new_table_expected
- primary_tables: `rag_diagnostics`, `rag_memory_sources`, `rag_project_doc_sources`, `rag_memory_text_units`, `rag_project_doc_text_units`

本阶段优先复用 RAG foundation 已创建的 `rag_diagnostics` 表和现有 Source/TextUnit 表。除非实现时发现缺少必要索引，否则不新增表。retry 直接更新对应 Corpus 的 TextUnit 索引字段，trace 与 warnings 统一进入 `rag_diagnostics`。

## Proposed Schema

本阶段不新增业务表。逻辑模型落在现有表：

- `rag_diagnostics`: 保存 `INDEXING_WARNING` 与 `SEARCH_TRACE`。
- `rag_memory_sources` / `rag_project_doc_sources`: debug 与 retry 的 Source 查找边界。
- `rag_memory_text_units` / `rag_project_doc_text_units`: retry fulltext/vector 的索引更新边界。

## Existing Schema Impact

- `rag_diagnostics` 已支持 `INDEXING_WARNING` 和 `SEARCH_TRACE`。
- `rag_diagnostics.expires_at` 已为 search trace TTL 预留索引。
- Memory/Project Doc TextUnit 表已包含 `text`、`search_text`、`fulltext_status`、embedding 字段和 `embedding_status`。
- Source 表已有 `status`、`scope_type`、`scope_code`、`source_code`、`source_version`。

## Diagnostics Records

### INDEXING_WARNING

- producer: ingest vector degrade、retry vector degrade、retry stage warning。
- required fields: `record_type`, `record_code`, `corpus_id`, `source_code`, `source_version`, `scope_type`, `scope_code`, `workspace_code`, `user_code`, `agent_code`, `stage_id`, `message`, `payload_json`, `created_at`。
- payload allowed fields: `code`, `stageId`, `status`, `updatedCount`, `failedCount`, `modelCode`, `errorCategory`。
- payload forbidden fields: provider 原始响应、API key、embedding、TextUnit 正文、Context Pack body。
- TTL: 不强制过期。

### SEARCH_TRACE

- producer: search request with `trace.enabled = true`。
- required fields: `record_type`, `record_code`, `profile_id`, `workspace_code`, `user_code`, `agent_code`, `query_text`, `payload_json`, `created_at`, `expires_at`。
- payload allowed fields: `routeCounts`, `routeTimingsMs`, `warnings`, `candidateRefs`, `resultCount`, `tokenUsage`, `contextStrategy`。
- candidateRefs shape: `corpusId`、`sourceCode`、`sourceVersion`、`targetType`、`targetCode`、`routeIds`。
- payload forbidden fields: hydrate 正文、Context Pack body、embedding、provider 原始响应、API key。
- TTL: `expires_at = created_at + 7 days`。

## Retry Updates

### Fulltext Retry

- Reads: selected Corpus TextUnits by `source_code`、`source_version`、`scope_type`、`scope_code`。
- Writes: `search_text` rebuilt from persisted `text`; `fulltext_status = INDEXED`; `updated_at` refreshed。
- Does not write: Source content, Source metadata, TextUnit `text`, embedding fields。

### Vector Retry

- Reads: selected Corpus TextUnits by `source_code`、`source_version`、`scope_type`、`scope_code`。
- Runtime: calls `LlmRuntimeService::embedding_call` per TextUnit or batch-equivalent local loop。
- Writes on success: `embedding_model_code`、`embedding_version`、`embedding_text_hash`、`embedding_dimension`、`embedding`、`embedding_status = INDEXED`、`updated_at`。
- Writes on failure: `embedding_status = FAILED` or leaves previous safe state according to stage policy；adds `INDEXING_WARNING` without raw provider payload。
- Does not write: TextUnit `text`、Source content、provider raw response。

## Debug Read Models

### Profile Debug

- Reads `rag_profiles` and related `rag_corpora` definitions。
- Returns profile summary, config summary and filter/binding metadata。
- Does not mutate any table。

### Source Debug

- Reads selected Source table by Corpus and direct source scope。
- Optionally reads TextUnit summaries。
- Reads `rag_diagnostics` warnings by `record_type = INDEXING_WARNING`、Corpus、Source code and version。
- Does not return embedding BLOB。
- Does not mutate any table。

## Query And Performance Plan

- Debug source uses existing `(source_code, source_version)` and `(scope_type, scope_code, status)` indexes。
- TextUnit retry uses existing `(source_code, source_version, ordinal)` indexes。
- Warning query uses `idx_rag_diagnostics_source`。
- Trace list or cleanup uses `idx_rag_diagnostics_expires`。
- Search trace write is one insert per traced search request。

## Data Governance

- diagnostics 是运行诊断数据，不反向驱动 Source 状态。
- Trace/Event payload 禁止保存正文、Context Pack body、embedding、provider 原始响应或 API key。
- Debug API 可以返回 TextUnit 摘要与必要文本诊断字段，但不能返回 embedding BLOB。
- Retry 不接收新正文，不重跑解析阶段，不绕过 Source/TextUnit scope。
- EventBus 发布失败不回滚已成功的数据库事务。

## Relationships And Cardinality

- 一个 Source version 可以有多个 TextUnit。
- 一个 Source version 可以有多个 `INDEXING_WARNING`。
- 一个 search 请求最多写入一个 `SEARCH_TRACE`。
- 一个 retry 请求可以产生多个 stage result，也可以产生多个 warning。
- diagnostics 不拥有 Source/TextUnit 生命周期，只引用 corpus/source/version/scope codes。

## Migration Plan

1. 默认不新增 migration，复用 `20260530_0022_create_rag_foundation.sql` 中的 `rag_diagnostics`。
2. 如实现发现必须补充索引，只允许新增幂等 `CREATE INDEX IF NOT EXISTS` migration。
3. 不执行 backfill。
4. rollback 只针对本阶段新增索引；不得删除 RAG 主数据表。

## Test Data And Fixtures

- Fixture 应覆盖 Memory 和 Project Doc Source/TextUnit。
- Fixture 应覆盖 `INDEXING_WARNING` debug 查询。
- Fixture 应覆盖 `SEARCH_TRACE` 写入和 TTL。
- Fake embedding runtime 应覆盖 vector retry 成功和失败。
- Fake EventBus 或 recording sink 应覆盖 safe payload 和 publish failure non-blocking。

## Validation Commands

- `make be-test`: 验证 debug/retry/trace/event 后端测试通过。
- `make be-check`: 验证后端编译与边界测试通过。
