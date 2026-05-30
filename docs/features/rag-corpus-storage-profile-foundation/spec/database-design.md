# Database Design: RAG 后端底座 - Corpus 存储与 Profile 基础

## Database Decision

- status: approved
- impact: create
- approved_by: langtaoxin
- approved_at: 2026-05-30

本 feature 新增 RAG 基础 schema。按最新确认：Corpus/Profile/diagnostics 继续合并；Source 和 TextUnit 按 Corpus 独立存储；全文检索和向量检索不再拆独立表，而是放在各 Corpus 自己的 TextUnit 表中。最终本 feature 计划新增 7 张表。

## Existing Schema Impact

- 不修改现有 user、workspace、model、chat、agent、graph、task、automation 表。
- 新增迁移文件建议命名为 `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`。
- 新增 migration 常量应注册到 `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`。
- 新增 SQLite repository 应沿用现有 `SqliteXxxRepository::connect(database_url)` 模式，在连接时执行 RAG migration。
- `GraphService`、Grafeo adapter 和 graph 相关表不受影响。

## Table Count

本设计新增 7 张表：

1. `rag_corpora`
2. `rag_profiles`
3. `rag_memory_sources`
4. `rag_project_doc_sources`
5. `rag_memory_text_units`
6. `rag_project_doc_text_units`
7. `rag_diagnostics`

本次不创建独立 FTS 表和 Vector 表。全文检索文本、全文索引状态、embedding 信息和向量 BLOB 存在各 Corpus 的 TextUnit 表中。

## Proposed Schema

### rag_corpora

- change_type: create
- purpose: 存 Corpus 基础配置、filterSchema、物理字段映射、表绑定和能力声明。
- fields:
  - `corpus_id TEXT PRIMARY KEY`
  - `display_name TEXT NOT NULL`
  - `status TEXT NOT NULL`
  - `description TEXT`
  - `capabilities_json TEXT NOT NULL`
  - `filter_schema_json TEXT NOT NULL`
  - `field_mapping_json TEXT NOT NULL`
  - `storage_binding_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - `status` 只允许 `enabled`、`reserved`、`disabled`。
  - 第一阶段 seed：`memory-card=enabled`、`project-doc=enabled`、`code-graph=reserved`。
  - `storage_binding_json` 第一阶段按 Corpus 指向各自 Source/TextUnit 表，以及统一 `rag_diagnostics`。
- indexes:
  - primary key covers `corpus_id`
- notes:
  - 原 `rag_corpus_profiles` 和 `rag_corpus_table_bindings` 合并为本表。
  - `code-graph` 只作为 reserved Corpus 存在，不能启用真实写入或检索。
  - `filter_schema_json` 保存逻辑字段到物理列的映射摘要；application 层仍需解析后执行类型和字段校验。

### rag_profiles

- change_type: create
- purpose: 统一存 Ingest Profile 与 Retrieval Profile。
- fields:
  - `profile_id TEXT PRIMARY KEY`
  - `profile_type TEXT NOT NULL`
  - `status TEXT NOT NULL`
  - `primary_corpus_id TEXT`
  - `corpus_ids_json TEXT NOT NULL`
  - `required_source_version TEXT`
  - `config_json TEXT NOT NULL`
  - `filters_json TEXT NOT NULL`
  - `degrade_policy TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - `profile_type` 只允许 `INGEST`、`RETRIEVAL`。
  - `status` 只允许 `enabled`、`reserved`、`disabled`。
  - `primary_corpus_id` 必须为空或引用已知 Corpus。
  - `corpus_ids_json` 必须只引用已知 Corpus。
  - `profile_type = RETRIEVAL` 时 `required_source_version` 必填；第一阶段没有 latest 版本语义。
  - `config_json` 中的 stage、retriever、fusion、dedup、rerank、hydrate、context ID 只能引用 Rust registry 中存在的 ID。
  - `GRAPH`、`SUMMARY_TREE` route 可在 reserved profile 中出现，但不能在 enabled profile 中启用执行。
- indexes:
  - `idx_rag_profiles_type_status` on `(profile_type, status)`
  - `idx_rag_profiles_primary_corpus` on `(primary_corpus_id, profile_type, status)`
- notes:
  - 原 `rag_ingest_profiles` 和 `rag_retrieval_profiles` 合并为本表。
  - `config_json` 按 profile_type 存不同结构：INGEST 存 DAG/stage config；RETRIEVAL 存 routes/fusion/dedup/rerank/hydrate/context。
  - 本 feature 只存储、读取和校验 Profile，不执行 DAG 或检索。

### rag_memory_sources

- change_type: create
- purpose: 存 `memory-card` 的 Source 元数据。
- fields:
  - `id INTEGER PRIMARY KEY`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `source_type TEXT NOT NULL`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `status TEXT NOT NULL`
  - `title TEXT`
  - `content_hash TEXT`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(source_code, source_version, scope_type, scope_code)`
  - `scope_type` 只允许 `USER`、`AGENT`、`SPACE`。
  - `status` 只允许 `PENDING`、`ACTIVE`、`FAILED`、`DISABLED`、`ARCHIVED`、`DELETED`。
- indexes:
  - `idx_rag_memory_sources_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_memory_sources_code_version` on `(source_code, source_version)`
- notes:
  - 表名已经表达 Corpus，因此表内不再重复存 `corpus_id`。
  - Memory 治理字段不在本 migration 中创建，后续 `Memory 与 Project Doc 领域字段` feature 通过 alter migration 添加到本表。

### rag_project_doc_sources

- change_type: create
- purpose: 存 `project-doc` 的 Source 元数据，并承载第一阶段 Project Doc Document 元数据。
- fields:
  - `id INTEGER PRIMARY KEY`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `source_type TEXT NOT NULL`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `status TEXT NOT NULL`
  - `title TEXT`
  - `content_hash TEXT`
  - `document_code TEXT`
  - `document_type TEXT`
  - `path TEXT`
  - `body_hash TEXT`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(source_code, source_version, scope_type, scope_code)`
  - `scope_type` 只允许 `USER`、`AGENT`、`SPACE`。
  - `status` 只允许 `PENDING`、`ACTIVE`、`FAILED`、`DISABLED`、`ARCHIVED`、`DELETED`。
- indexes:
  - `idx_rag_project_doc_sources_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_project_doc_sources_code_version` on `(source_code, source_version)`
  - `idx_rag_project_doc_sources_document_code` on `(document_code)`
- notes:
  - 表名已经表达 Corpus，因此表内不再重复存 `corpus_id`。
  - Project Doc 第一阶段按一个 Source 对应一个文档处理，因此 Document 元数据直接放在 Source 行上。
  - Project Doc 的 repo、module、doc_type 细化等领域字段在后续领域字段 feature 添加到本表。

### rag_memory_text_units

- change_type: create
- purpose: 存 `memory-card` 的 TextUnit 主数据、全文检索文本和向量检索数据。
- fields:
  - `id INTEGER PRIMARY KEY`
  - `text_unit_code TEXT NOT NULL UNIQUE`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `unit_type TEXT NOT NULL`
  - `text TEXT NOT NULL`
  - `search_text TEXT NOT NULL`
  - `heading_path TEXT NOT NULL`
  - `ordinal INTEGER NOT NULL`
  - `token_count INTEGER`
  - `content_hash TEXT`
  - `status TEXT NOT NULL`
  - `fulltext_status TEXT NOT NULL`
  - `embedding_model_code TEXT`
  - `embedding_version TEXT`
  - `embedding_text_hash TEXT`
  - `embedding_dimension INTEGER`
  - `embedding BLOB`
  - `embedding_status TEXT NOT NULL`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(source_code, source_version, ordinal)`
  - `unit_type` 只允许 `LEAF`、`SUMMARY`、`SYMBOL`、`NOTE`。
  - `status` 只允许 `ACTIVE`、`DISABLED`、`DELETED`。
  - `fulltext_status` 只允许 `PENDING`、`INDEXED`、`FAILED`、`SKIPPED`。
  - `embedding_status` 只允许 `PENDING`、`INDEXED`、`FAILED`、`SKIPPED`。
  - 当 `embedding_status = INDEXED` 时，`embedding_model_code`、`embedding_version`、`embedding_text_hash`、`embedding_dimension` 和 `embedding` 必须有值。
- indexes:
  - `idx_rag_memory_text_units_source` on `(source_code, source_version, ordinal)`
  - `idx_rag_memory_text_units_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_memory_text_units_fulltext_status` on `(fulltext_status)`
  - `idx_rag_memory_text_units_embedding_model` on `(embedding_model_code, embedding_version, embedding_status)`
- notes:
  - 全文检索直接使用 `search_text` 和本表元数据，不单独创建 FTS 物理表。
  - 向量检索直接读取本表 `embedding` BLOB，由后端 brute-force cosine 计算。
  - 第一阶段只支持每个 TextUnit 一份当前 Profile 使用的 embedding。

### rag_project_doc_text_units

- change_type: create
- purpose: 存 `project-doc` 的 TextUnit 主数据、全文检索文本和向量检索数据。
- fields:
  - `id INTEGER PRIMARY KEY`
  - `text_unit_code TEXT NOT NULL UNIQUE`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `document_code TEXT`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `unit_type TEXT NOT NULL`
  - `text TEXT NOT NULL`
  - `search_text TEXT NOT NULL`
  - `heading_path TEXT NOT NULL`
  - `ordinal INTEGER NOT NULL`
  - `token_count INTEGER`
  - `content_hash TEXT`
  - `status TEXT NOT NULL`
  - `fulltext_status TEXT NOT NULL`
  - `embedding_model_code TEXT`
  - `embedding_version TEXT`
  - `embedding_text_hash TEXT`
  - `embedding_dimension INTEGER`
  - `embedding BLOB`
  - `embedding_status TEXT NOT NULL`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(source_code, source_version, ordinal)`
  - `unit_type` 只允许 `LEAF`、`SUMMARY`、`SYMBOL`、`NOTE`。
  - `status` 只允许 `ACTIVE`、`DISABLED`、`DELETED`。
  - `fulltext_status` 只允许 `PENDING`、`INDEXED`、`FAILED`、`SKIPPED`。
  - `embedding_status` 只允许 `PENDING`、`INDEXED`、`FAILED`、`SKIPPED`。
  - 当 `embedding_status = INDEXED` 时，`embedding_model_code`、`embedding_version`、`embedding_text_hash`、`embedding_dimension` 和 `embedding` 必须有值。
- indexes:
  - `idx_rag_project_doc_text_units_source` on `(source_code, source_version, ordinal)`
  - `idx_rag_project_doc_text_units_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_project_doc_text_units_document` on `(document_code)`
  - `idx_rag_project_doc_text_units_fulltext_status` on `(fulltext_status)`
  - `idx_rag_project_doc_text_units_embedding_model` on `(embedding_model_code, embedding_version, embedding_status)`
- notes:
  - 全文检索直接使用 `search_text` 和本表元数据，不单独创建 FTS 物理表。
  - 向量检索直接读取本表 `embedding` BLOB，由后端 brute-force cosine 计算。
  - 与 Memory TextUnit 表相比，本表保留 `document_code`，方便 Project Doc hydrate 和文档级过滤。

### rag_diagnostics

- change_type: create
- purpose: 统一存 indexing warnings 和 lightweight search trace。
- fields:
  - `id INTEGER PRIMARY KEY`
  - `record_type TEXT NOT NULL`
  - `record_code TEXT NOT NULL UNIQUE`
  - `corpus_id TEXT`
  - `profile_id TEXT`
  - `source_code TEXT`
  - `source_version TEXT`
  - `scope_type TEXT`
  - `scope_code TEXT`
  - `workspace_code TEXT`
  - `user_code TEXT`
  - `agent_code TEXT`
  - `stage_id TEXT`
  - `query_text TEXT`
  - `message TEXT`
  - `payload_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `expires_at INTEGER`
- constraints:
  - `record_type` 只允许 `INDEXING_WARNING`、`SEARCH_TRACE`。
  - `INDEXING_WARNING` 不要求 `expires_at`。
  - `SEARCH_TRACE` 必须有 `profile_id`、`workspace_code`、`query_text` 和 `expires_at`。
  - diagnostics 不保存 hydrate 正文、Context Pack body、embedding、provider 原始响应或 API key。
- indexes:
  - `idx_rag_diagnostics_type_created` on `(record_type, created_at DESC)`
  - `idx_rag_diagnostics_source` on `(record_type, corpus_id, source_code, source_version)`
  - `idx_rag_diagnostics_workspace` on `(record_type, workspace_code, created_at DESC)`
  - `idx_rag_diagnostics_expires` on `(record_type, expires_at)`
- notes:
  - 原 `rag_indexing_warnings` 和 `rag_search_traces` 合并为本表。
  - 本 feature 只建最小表壳。真实 warning 写入、trace 写入、TTL 清理和 debug 展示属于后续诊断 feature。

## Merge Decisions

### 已合并

- `rag_corpus_profiles` + `rag_corpus_table_bindings` -> `rag_corpora`
  - reason: Corpus 配置、filterSchema、field mapping 和 storage binding 是一对一关系，生命周期一致。
- `rag_ingest_profiles` + `rag_retrieval_profiles` -> `rag_profiles`
  - reason: 两者都是 Profile 配置，差异可以通过 `profile_type` 和 `config_json` 表达。
- `rag_indexing_warnings` + `rag_search_traces` -> `rag_diagnostics`
  - reason: 两者都属于 RAG 运行诊断数据，可以通过 `record_type` 区分，差异字段放在通用列和 `payload_json` 中。
- 每个 Corpus 内部的 TextUnit + fulltext + vector -> `rag_memory_text_units` / `rag_project_doc_text_units`
  - reason: 当前 RAG 模块支持全文检索和向量检索在一张表中；按 Corpus 分表后，全文和向量仍保留在对应 Corpus 的 TextUnit 行上。

### 已拆分或暂不合并

- `rag_memory_sources` 与 `rag_project_doc_sources` 不合并
  - reason: Memory 和 Project Doc 后续 filterable 字段会明显分化。按 Corpus 分开可以减少稀疏字段，降低后续 alter migration 影响面，并保留独立索引策略。
- `rag_memory_text_units` 与 `rag_project_doc_text_units` 不合并
  - reason: 两类 TextUnit 的切分策略、文档 hydrate、heading/path 字段和后续 filterable 字段会分化。按 Corpus 分开更利于独立演进，同时仍在各自表内合并全文和向量数据。
- Project Doc Document 不单独拆表
  - reason: 第一阶段按一个 Project Doc Source 对应一个文档处理，Document 元数据放在 `rag_project_doc_sources` 即可。若未来一个 Source 对应多个 Document，再单独评估拆表。
- `rag_corpora` 不合并进 `rag_profiles`
  - reason: Corpus 是知识域定义，Profile 是执行配置；一个 Corpus 有多个 Profile，Retrieval Profile 还可能跨多个 Corpus。
- `rag_profiles` 不合并进 Source 表
  - reason: Profile 是配置数据，Source 是业务数据；生命周期、查询方式、权限边界不同。
- Source 表不合并进 TextUnit 表
  - reason: Source 与 TextUnit 是一对多关系。合并会在每个 TextUnit 上重复 Source 状态、标题、hash、metadata，Source 状态更新和 hydrate 校验会变复杂。
- `rag_diagnostics` 不合并进 Source 或 TextUnit 表
  - reason: diagnostics 是运行诊断数据，包含 warning/trace 和 TTL；合并到热检索表会污染查询路径，并增加误返回诊断内容的风险。

### 需要用户判断的未来分歧

- 如果后续必须使用 SQLite FTS5 的 `MATCH` 语义和 FTS5 排名能力，SQLite 通常需要额外的 FTS virtual table。当前设计先按“各 Corpus 的 TextUnit 表同时承载全文与向量数据”处理，不创建 FTS virtual table。
- 如果后续同一 TextUnit 需要同时保留多个 embedding 模型或多个 embedding version，单行 embedding 字段会不够，需要新增 embedding 子表或改为多行版本化策略。当前 PRD 已规定 Profile 固定 `embeddingModelCode`，所以第一阶段先不拆表。

## Relationships And Cardinality

- 一个 Corpus 对应一个 `rag_corpora` 记录。
- 一个 Corpus 可以有多个 `rag_profiles`；一个 Retrieval Profile 可以覆盖多个 Corpus。
- 一个 Memory Source 由 `rag_memory_sources(source_code, source_version, scope_type, scope_code)` 唯一识别。
- 一个 Project Doc Source 由 `rag_project_doc_sources(source_code, source_version, scope_type, scope_code)` 唯一识别。
- 一个 Source 可以有多个 TextUnit。
- 一个 Project Doc Source 第一阶段对应一个文档，文档字段存放在 `rag_project_doc_sources`。
- 一个 TextUnit 第一阶段最多保存一份 active embedding。
- diagnostics 是运行诊断数据，不反向驱动 Source 状态。

## Migration Plan

1. 新增 `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`。
2. 在 `sqlite/migrations.rs` 中新增 `RAG_FOUNDATION_MIGRATION` 常量。
3. 新增 `SqliteRagRepository::connect(database_url)`，连接时执行 RAG migration。
4. migration 使用 `CREATE TABLE IF NOT EXISTS`、`CREATE INDEX IF NOT EXISTS` 和 `INSERT OR IGNORE` seed，保证重复执行幂等。
5. seed `memory-card`、`project-doc`、`code-graph` Corpus。
6. seed 第一阶段最小 Ingest/Retrieval Profile。Profile 可先为后续 ingest/search 提供配置骨架，真实执行由后续 feature 实现。
7. 不执行数据 backfill，因为这是新 schema。
8. rollback 以删除新增 RAG 表为主；已有业务表不受影响。

## Query And Performance Plan

- Corpus 查询按 `corpus_id` 主键读取。
- Profile 查询按 `profile_id` 主键读取，按 `(profile_type, status)` 列表查询。
- filterSchema 校验先按 Corpus 读取 `filter_schema_json`，再在内存中解析。
- Source 查询按目标 Corpus 选择 `rag_memory_sources` 或 `rag_project_doc_sources`，主要使用 `(source_code, source_version)` 和 `(scope_type, scope_code, status)` 索引。
- TextUnit 查询按目标 Corpus 选择 `rag_memory_text_units` 或 `rag_project_doc_text_units`，主要使用 `(source_code, source_version, ordinal)` 和 `(scope_type, scope_code, status)` 索引。
- 全文检索第一阶段基于对应 TextUnit 表的 `search_text` 与 scope/source/status 条件完成，不使用单独 FTS 表。
- Vector 查询第一版由后端按 scope/source/model/status 过滤后，读取对应 TextUnit 表的 `embedding` BLOB 并执行 brute-force cosine；后续向量存储替换不改变 application 契约。
- diagnostics 查询按 `record_type`、source、workspace 和 created_at 排序。
- search trace TTL 清理按 `record_type = SEARCH_TRACE` 和 `expires_at` 索引。

## Data Governance

- 所有主数据表保留 `scope_type` 和 `scope_code`。
- HTTP workspace access guard 不在数据库层完成；数据库层只保存可过滤的 scope 字段。
- 普通检索后续只读取 active Source 与 active TextUnit。
- metadata 只保存低频扩展信息，不能作为未声明过滤字段绕过 filterSchema。
- diagnostics、Event payload 不保存正文、Context Pack body、embedding、provider 原始响应或 API key。
- `embedding` BLOB 属于索引数据，只能通过 RAG repository 读写，不能通过 debug API 直接返回。
- `SEARCH_TRACE` 的 TTL 策略为 7 天，具体清理任务由后续诊断 feature 实现。

## Test Data And Fixtures

- SQLite in-memory fixture 应能执行 RAG migration 并 seed 三个 Corpus。
- Fixture 应包含 enabled Corpus：`memory-card`、`project-doc`。
- Fixture 应包含 reserved Corpus：`code-graph`。
- Profile fixture 应覆盖 enabled、reserved、disabled 三种状态。
- filterSchema fixture 应覆盖合法字段、未知字段、类型不匹配字段和跨 Corpus 字段。
- Memory TextUnit fixture 应在 `rag_memory_text_units` 同一行覆盖 `search_text` 和 `embedding` 字段。
- Project Doc TextUnit fixture 应在 `rag_project_doc_text_units` 同一行覆盖 `search_text`、`embedding` 和 `document_code` 字段。
- Vector fixture 可使用短 BLOB 测试编码/解码边界，不需要真实模型调用。
- diagnostics fixture 应覆盖 `INDEXING_WARNING` 和 `SEARCH_TRACE` 两种 `record_type`。

## Validation Commands

- `make be-test`: 验证 RAG migration、repository、filterSchema 和 seed 数据测试通过；expect: 后端测试通过。
- `make be-check`: 验证新增 RAG domain/application/persistence 模块可以编译；expect: backend compile check 通过。
- `make be-test`: 验证 SQLite in-memory repository 可重复执行 migration；expect: 重复 connect 不产生重复 seed 或 schema 错误。
