# Database Design: RAG 后端底座 - 同步写入 DAG

## Database Decision

- status: approved
- impact: seed_and_repository
- approved_by: langtaoxin
- approved_at: 2026-05-31

本阶段优先复用现有 RAG 表结构，不新增 Source/TextUnit 业务表，不拆出独立全文或向量索引表。数据库变更集中在内置 Ingest Profile seed、repository 写入方法、diagnostics warning 写入和少量辅助索引。

## Existing Schema Impact

- 依赖 `rag_corpora`、`rag_profiles`、`rag_memory_sources`、`rag_project_doc_sources`、`rag_memory_text_units`、`rag_project_doc_text_units`、`rag_diagnostics`。
- `rag_memory_sources` 已有 Memory governance fields。
- `rag_project_doc_sources` 已有 Project Doc repo/module fields。
- `rag_memory_text_units` 和 `rag_project_doc_text_units` 已合并全文和向量字段。
- `rag_profiles` 当前已有 retrieval profiles，需要增加 ingest profiles。
- 不修改 user、workspace、model、graph、chat、agent、task、automation 表。

## Proposed Schema

### rag_profiles seed

- change_type: upsert seed
- new_profiles:
  - `memory-card-ingest-v1`
  - `project-doc-ingest-v1`
- profile_type: `INGEST`
- status: `enabled`
- primary_corpus_id: 对应 Corpus
- corpus_ids_json: 单 Corpus 数组
- required_source_version: `null`
- degrade_policy:
  - memory-card: `FAIL_FAST`
  - project-doc: `SKIP_VECTOR_WITH_WARNING`
- config_json:
  - `dag.nodes`
  - `dag.edges`
  - `stages`
  - `embeddingModelCode`
  - `embeddingVersion`
  - `splitter`

### rag_memory_sources write fields

- stable_key: `source_code + source_version + scope_type + scope_code`
- write_fields:
  - common Source fields
  - Memory domain fields from payload
  - `status`
  - `metadata`
- status_flow:
  - `PENDING`
  - `ACTIVE`
  - `FAILED`

### rag_project_doc_sources write fields

- stable_key: `source_code + source_version + scope_type + scope_code`
- write_fields:
  - common Source fields
  - Project Doc domain fields from payload
  - `status`
  - `metadata`
- status_flow:
  - `PENDING`
  - `ACTIVE`
  - `FAILED`

### TextUnit tables

- stable_key: `source_code + source_version + ordinal`
- write_fields:
  - `text_unit_code`
  - `source_code`
  - `source_version`
  - `scope_type`
  - `scope_code`
  - `document_code` for Project Doc
  - `unit_type`
  - `text`
  - `search_text`
  - `heading_path`
  - `ordinal`
  - `token_count`
  - `content_hash`
  - `status`
  - `fulltext_status`
  - `embedding_model_code`
  - `embedding_version`
  - `embedding_text_hash`
  - `embedding_dimension`
  - `embedding`
  - `embedding_status`
  - `metadata`

### rag_diagnostics warnings

- record_type: `INDEXING_WARNING`
- corpus_id、profile_id、source_code、source_version、scope_type、scope_code、stage_id
- message: 安全摘要
- payload_json: 不含正文、provider 原始响应、密钥、embedding blob
- expires_at: optional

## Relationships And Cardinality

- 一个 Source version 对应多个 TextUnit。
- 一个 Project Doc Source 可有一个 `document_code`，TextUnit 通过 `document_code` 关联文档逻辑。
- 一个 Source version 可以有多个 warning。
- 同一 `source_code` 可以有多个 `source_version` 共存。
- TextUnit ordinal 在同一 Source version 内稳定。

## Migration Plan

- 新增迁移文件建议命名为 `tron-backend/migrations/20260531_0024_add_rag_ingest_profiles.sql`。
- 在 `migrations.rs` 注册 migration 常量。
- `SqliteRagRepository::connect` 在 foundation 与 domain fields migration 后执行 ingest profile seed。
- 不新增业务表；如需辅助索引，使用 `CREATE INDEX IF NOT EXISTS`。
- Repository 写入必须使用 transaction。
- 写入 Source 时先 upsert `PENDING`，完成后再更新为 `ACTIVE` 或 `FAILED`。
- 重试同版本写入时，先 upsert Source，再按 stable key upsert TextUnit，避免重复数据。

## Query And Performance Plan

- Source upsert 使用现有唯一约束。
- TextUnit upsert 使用现有唯一约束。
- Ingest 查询只按 source stable key、profileId、corpusId 读取。
- 不在本阶段做 search route 查询。
- Markdown splitter 控制 TextUnit 数量，避免单次写入过多行。
- embedding 调用逐 TextUnit 执行，第一版同步完成。

## Data Governance

- `scope_type` 和 `scope_code` 是 RAG 存储隔离字段。
- `workspaceCode` 用于 HTTP access guard 和 SPACE scope，不替代 USER/AGENT scope。
- Warning 不保存正文、Context Pack body、provider 原始响应、API key 或 embedding blob。
- RAG 不修改 Memory 治理状态语义，只保存调用方传入字段。
- RAG 不执行 repo、role、field 或内容脱敏权限。

## Test Data And Fixtures

- Memory fixture: one plain text memory card with `memoryType=RULE`、`reviewStatus=APPROVED`、`sourceVersion=v1`。
- Project Doc fixture: Markdown document with two headings and `repoCode=REPO-TRON`、`moduleCode=MOD-RAG`。
- Embedding fixture: fake `LlmRuntimeService` 返回固定向量。
- Degrade fixture: fake embedding failure for Project Doc returns warning and active Source。
- Failure fixture: fake embedding failure for Memory returns failed Source。

## Validation Commands

- `make be-test`
- `make be-check`
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-ingest-dag --phase final`
