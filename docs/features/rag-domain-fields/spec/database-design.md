# Database Design: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Database Decision

- status: approved
- impact: alter
- approved_by: langtaoxin
- approved_at: 2026-05-30

本 feature 演进 RAG 既有 schema，不新增表。新增字段都落在 Corpus 对应的 Source 表；TextUnit 表保持不变。`rag_corpora` 的 seed 会更新 `filter_schema_json` 和 `field_mapping_json`，让 application 层能识别新 logical fields。

## Existing Schema Impact

- 依赖已存在的 `rag_memory_sources`、`rag_project_doc_sources`、`rag_corpora`。
- 新增迁移文件建议命名为 `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`。
- 新增 migration 常量注册到 `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`。
- `SqliteRagRepository` 初始化时先执行 foundation migration，再按列存在性执行 domain fields migration。
- 不修改 user、workspace、model、chat、agent、graph、task、automation 表。
- 不新增 RAG HTTP route，也不引入 GraphService、LlmRuntimeService 或 EventBus 依赖。

## Proposed Schema

### rag_memory_sources

- change_type: alter
- purpose: 存 Memory 治理字段，供后续 ingest 持久化、search 过滤、hydrate 返回。
- add_fields:
  - `memory_type TEXT`
  - `memory_status TEXT`
  - `review_status TEXT`
  - `version TEXT`
  - `superseded_by TEXT`
  - `conflict_group TEXT`
  - `duplicate_group TEXT`
  - `injectable INTEGER NOT NULL DEFAULT 1`
  - `importance_score INTEGER`
  - `confidence_score INTEGER`
  - `quality_score INTEGER`
  - `pin_level INTEGER NOT NULL DEFAULT 0`
  - `expires_at INTEGER`
  - `derived_source_type TEXT`
  - `derived_source_code TEXT`
- constraints:
  - `injectable` 只能表达 boolean storage：0 或 1。
  - score 字段按整数分值保存，默认不在数据库层限制区间。
  - RAG 不根据 `memory_status`、`review_status` 或 `version` 自动流转业务状态。
- indexes:
  - `idx_rag_memory_sources_domain_status` on `(memory_type, memory_status, review_status, injectable)`
  - `idx_rag_memory_sources_domain_rank` on `(pin_level, importance_score, confidence_score, quality_score)`
  - `idx_rag_memory_sources_expires` on `(expires_at)`
- notes:
  - 审核原因、质量门禁详情、冲突详情、重复详情、派生链路详情、reviewer code 和 audit hints 继续放在 `metadata`。

### rag_project_doc_sources

- change_type: alter
- purpose: 存 Project Doc 仓库和模块字段，供后续 ingest/search/hydrate 使用。
- add_fields:
  - `repo_code TEXT`
  - `module_code TEXT`
  - `module_name TEXT`
- existing_fields_reused:
  - `document_code`
  - `document_type`
  - `path`
  - `title`
  - `body_hash`
- constraints:
  - `documentType` logical field 映射到已有 `document_type`。
  - Project Doc 标签、文档摘要扩展和导入细节继续进入 `metadata`，不在本切片过滤。
- indexes:
  - `idx_rag_project_doc_sources_repo_module` on `(repo_code, module_code, document_type)`
  - `idx_rag_project_doc_sources_path` on `(path)`

### rag_corpora seed

- change_type: update
- purpose: 更新 `memory-card` 与 `project-doc` 的 filterSchema 和 fieldMapping。
- memory_filter_fields:
  - `scopeType -> scope_type`
  - `scopeCode -> scope_code`
  - `sourceCode -> source_code`
  - `sourceVersion -> source_version`
  - `status -> status`
  - `memoryType -> memory_type`
  - `memoryStatus -> memory_status`
  - `reviewStatus -> review_status`
  - `version -> version`
  - `supersededBy -> superseded_by`
  - `conflictGroup -> conflict_group`
  - `duplicateGroup -> duplicate_group`
  - `injectable -> injectable`
  - `importanceScore -> importance_score`
  - `confidenceScore -> confidence_score`
  - `qualityScore -> quality_score`
  - `pinLevel -> pin_level`
  - `expiresAt -> expires_at`
  - `derivedSourceType -> derived_source_type`
  - `derivedSourceCode -> derived_source_code`
- project_doc_filter_fields:
  - `scopeType -> scope_type`
  - `scopeCode -> scope_code`
  - `sourceCode -> source_code`
  - `sourceVersion -> source_version`
  - `status -> status`
  - `documentCode -> document_code`
  - `documentType -> document_type`
  - `path -> path`
  - `repoCode -> repo_code`
  - `moduleCode -> module_code`
  - `moduleName -> module_name`

## Relationships And Cardinality

- `rag_memory_sources` 仍由 `(source_code, source_version, scope_type, scope_code)` 唯一定位一个 Memory Source 版本。
- `rag_project_doc_sources` 仍由 `(source_code, source_version, scope_type, scope_code)` 唯一定位一个 Project Doc Source 版本。
- 新增领域字段不改变 Source 与 TextUnit 的关系。
- `rag_corpora` 到 Source/TextUnit 表的 storage binding 不变。

## Migration Plan

1. 新增 `RAG_DOMAIN_FIELDS_MIGRATION` 常量。
2. 在 `SqliteRagRepository` 中加入 `apply_migrations` 或等价方法。
3. 初始化顺序为 foundation migration、domain field column checks、domain field seed update。
4. 对每个新增列使用 `PRAGMA table_info` 风格检查；缺列时执行对应 `ALTER TABLE ADD COLUMN`。
5. 索引使用 `CREATE INDEX IF NOT EXISTS`，可安全重复执行。
6. seed update 使用 `INSERT ... ON CONFLICT DO UPDATE` 或针对 `rag_corpora` 的 `UPDATE`，保持幂等。

## Query And Performance Plan

- Memory 常用过滤会命中 `memory_type`、`memory_status`、`review_status`、`injectable`、`pin_level` 和 score 字段索引。
- Project Doc 常用过滤会命中 `repo_code`、`module_code`、`document_type` 和 `path`。
- 本阶段不构建 SQL predicate；只验证字段映射与类型。后续 search feature 再把 normalized filters 转为查询条件。
- 本阶段不修改 TextUnit 表索引，因为领域字段位于 Source 层。

## Data Governance

- Memory 治理字段只保存调用方传入值，不执行业务状态流转。
- metadata 不参与过滤；需要过滤的字段必须提升为物理列并进入 filterSchema。
- 新增字段不得保存正文、embedding、provider 原始响应、API key 或权限策略。
- Project Doc 的 repo/module 字段只用于知识来源定位，不等价于 repo 权限。

## Test Data And Fixtures

- 使用 SQLite `:memory:` repository 执行 foundation + domain fields migration。
- 通过 schema inspection 断言新增列存在。
- 通过 `RagService::get_corpus` 断言 fieldMapping/filterSchema 包含新增 logical fields。
- 通过 `RagService::validate_filters` 断言 Memory 与 Project Doc 新字段可验证，未知字段和类型错误仍被拒绝。

## Validation Commands

- `make be-test`
- `make be-check`
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-domain-fields --phase spec --spec-stage test-plan --complete-spec --no-downstream-artifacts`
