# Database Design: RAG 后端底座 - 检索编排与 Context Pack

## Database Decision

- status: approved
- impact: seed_and_repository
- approved_by: langtaoxin
- approved_at: 2026-05-31

本阶段复用现有 RAG SQLite 表，不新增业务表，不拆分全文或向量索引表。数据库工作集中在 Retrieval Profile 配置补齐、repository search 读方法、必要查询索引和测试 fixture。

## Existing Schema Impact

- 依赖 `rag_corpora`、`rag_profiles`、`rag_memory_sources`、`rag_project_doc_sources`、`rag_memory_text_units`、`rag_project_doc_text_units`。
- `rag_profiles` 已有 `memory-related-v1`、`project-doc-related-v1` retrieval profiles。
- `rag_memory_text_units` 和 `rag_project_doc_text_units` 已包含 `search_text`、`fulltext_status`、`embedding_model_code`、`embedding_version`、`embedding_dimension`、`embedding`、`embedding_status`。
- `rag_memory_sources` 和 `rag_project_doc_sources` 已包含领域过滤字段。
- 不修改 user、workspace、model、graph、chat、agent、task、automation 表。

## Proposed Schema

### rag_profiles seed

- change_type: upsert seed adjustment
- profiles:
  - `memory-related-v1`
  - `project-doc-related-v1`
- profile_type: `RETRIEVAL`
- status: `enabled`
- required_source_version: 固定版本，不能为 `latest` 或空值
- config_json:
  - `routes`
  - `fusionPolicy`
  - `dedupPolicy`
  - `rerankPolicy`
  - `hydratePolicy`
  - `contextPolicy`
  - `embeddingModelCode`
  - `rerankModelCode`
- filters_json: Profile 默认过滤条件
- degrade_policy:
  - vector route 不可用时可按 Profile 降级为 fulltext-only

### Search Read Models

不新增持久表，但 repository 需要返回以下 read model：

- Source summary: source code、source version、scope、status、title、domain fields、metadata。
- TextUnit hit: text unit code、source code、source version、scope、text、search text、heading path、ordinal、token count、embedding metadata、metadata。
- CandidateHit: application 层统一结构，不落库。
- ContextPack: application 层组装结果，不落库。

### Optional Indexes

如现有索引不足，可以添加：

- `rag_memory_text_units(source_version, scope_type, scope_code, status, fulltext_status)`
- `rag_project_doc_text_units(source_version, scope_type, scope_code, status, fulltext_status)`
- `rag_memory_text_units(embedding_model_code, embedding_version, embedding_status)`
- `rag_project_doc_text_units(embedding_model_code, embedding_version, embedding_status)`

索引必须使用 `CREATE INDEX IF NOT EXISTS`。

## Relationships And Cardinality

- 一个 Retrieval Profile 可查询一个或多个 Corpus。
- 一个 Source version 对应多个 TextUnit。
- 一个 query 可生成多条 route 命中，同一 TextUnit 多 route 命中通过 dedupKey 合并。
- Hydrate 只返回 Source 为 `ACTIVE` 的命中。
- Context Pack 只是响应结构，不保存到数据库。

## Migration Plan

- 如只需调整 seed，新建迁移文件建议命名为 `tron-backend/migrations/20260531_0025_add_rag_search_profiles.sql`。
- 在 `migrations.rs` 注册 migration 常量。
- `SqliteRagRepository::connect` 在 ingest profile seed 后执行 search profile seed。
- repository 查询必须使用参数化 SQL。
- 不写入 search trace，不写入 EventBus 事件表。

## Query And Performance Plan

- Fulltext MVP 可基于 `search_text LIKE` 或 SQLite 可用全文能力实现，结果按简单相关度和 ordinal 排序。
- Vector MVP 从 TextUnit embedding blob 解码，计算 cosine similarity，适合第一阶段小数据量。
- 每个 route 使用 `topK` 限制候选数量。
- Fusion 使用 Weighted RRF 或等价简化实现。
- Hydrate 批量读取 Source，避免每条命中单独查询。
- Context assembly 按 tokenBudget 截断，避免响应无限增长。

## Data Governance

- Search 只读 `ACTIVE` Source 和可见 TextUnit。
- `scope_type` 和 `scope_code` 是 RAG 存储隔离字段。
- HTTP workspace access guard 先于 RAG search 执行。
- Request filters 不能放宽 Profile filters。
- Search 响应可以返回正文片段，但本阶段不把 trace、context pack body 或 provider 原始响应落库。
- RAG 不执行 repo、role、field 或内容脱敏权限。

## Test Data And Fixtures

- Memory fixture: ingest 两条 `memory-card`，覆盖 fulltext、vector、dedup 和 `MEMORY_CONTEXT_PACK`。
- Project Doc fixture: ingest Markdown 多 heading 文档，覆盖 `DOC_QA_CONTEXT_PACK`。
- Version fixture: 同一 sourceCode 写入 `v1` 和 `v2`，search profile 固定 `v1`，请求 filter 可切到 `v2`。
- Filter fixture: Memory 使用 `memoryType`、`injectable`；Project Doc 使用 `repoCode`、`moduleCode`、`documentType`。
- Vector fixture: fake embedding runtime 返回可预测向量。
- Rerank fixture: fake rerank runtime 返回可预测排序。

## Validation Commands

- `make be-test`
- `make be-check`
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-search-context-pack --phase final`
