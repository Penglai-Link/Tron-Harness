# Database Design: RAG 后端底座 - Corpus 存储与 Profile 基础

## Database Decision

- status: draft
- impact: create
- approved_by: none
- approved_at: none

本 feature 需要新增 RAG 基础 schema。迁移只建立 Corpus/Profile/Binding/filterSchema、通用 Source/Document/TextUnit/FTS/Vector 表边界，以及 warnings/trace 的最小表壳。它不写入真实业务数据，不执行 ingest/search，不添加 Memory 治理字段或 Project Doc 领域字段。

## Existing Schema Impact

- 不修改现有 user、workspace、model、chat、agent、graph、task、automation 表。
- 新增迁移文件建议命名为 `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`。
- 新增 migration 常量应注册到 `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`。
- 新增 SQLite repository 应沿用现有 `SqliteXxxRepository::connect(database_url)` 模式，在连接时执行 RAG migration。
- `GraphService`、Grafeo adapter 和 graph 相关表不受影响。

## Proposed Schema

### rag_corpus_profiles

- change_type: create
- fields:
  - `corpus_id TEXT PRIMARY KEY`
  - `display_name TEXT NOT NULL`
  - `status TEXT NOT NULL`
  - `description TEXT`
  - `capabilities_json TEXT NOT NULL`
  - `filter_schema_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - `status` 只允许 `enabled`、`reserved`、`disabled`。
  - 第一阶段 seed：`memory-card=enabled`、`project-doc=enabled`、`code-graph=reserved`。
- indexes:
  - primary key covers `corpus_id`
- notes:
  - `code-graph` 只作为 reserved Corpus 存在，不能启用真实写入或检索。
  - `filter_schema_json` 保存逻辑字段到物理列的映射摘要；application 层仍需解析后执行类型和字段校验。

### rag_corpus_table_bindings

- change_type: create
- fields:
  - `corpus_id TEXT PRIMARY KEY`
  - `source_table TEXT NOT NULL`
  - `document_table TEXT`
  - `text_unit_table TEXT`
  - `fts_table TEXT`
  - `vector_table TEXT`
  - `summary_tree_table TEXT`
  - `graph_node_table TEXT`
  - `graph_edge_table TEXT`
  - `field_mapping_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - `corpus_id` references `rag_corpus_profiles(corpus_id)` logically; SQLite 可不强制外键，但 repository 必须校验。
  - `memory-card` 与 `project-doc` 必须有 source/textUnit/fts/vector 表名。
  - `code-graph` 保持 reserved，不得配置可执行 graph node/edge 表。
- indexes:
  - primary key covers `corpus_id`
- notes:
  - `summary_tree_table`、`graph_node_table`、`graph_edge_table` 在本阶段为空或 unsupported。
  - 后续替换存储引擎时，application 层继续通过 binding 获取逻辑落点，不直接硬编码物理表。

### rag_ingest_profiles

- change_type: create
- fields:
  - `profile_id TEXT PRIMARY KEY`
  - `corpus_id TEXT NOT NULL`
  - `status TEXT NOT NULL`
  - `dag_json TEXT NOT NULL`
  - `stage_config_json TEXT NOT NULL`
  - `degrade_policy TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - `status` 只允许 `enabled`、`reserved`、`disabled`。
  - `corpus_id` 必须引用已知 Corpus。
  - `dag_json` 只能引用 Rust strategy registry 中存在的 stage strategy ID。
- indexes:
  - `idx_rag_ingest_profiles_corpus_status` on `(corpus_id, status)`
- notes:
  - 本 feature 只存储和读取 profile，不执行 DAG。
  - `SUMMARY_TREE_BUILDER`、Graph 相关 stage 可在 registry 中标记 reserved，但不可执行。

### rag_retrieval_profiles

- change_type: create
- fields:
  - `profile_id TEXT PRIMARY KEY`
  - `status TEXT NOT NULL`
  - `corpus_ids_json TEXT NOT NULL`
  - `required_source_version TEXT NOT NULL`
  - `filters_json TEXT NOT NULL`
  - `routes_json TEXT NOT NULL`
  - `fusion_policy TEXT NOT NULL`
  - `dedup_policy TEXT NOT NULL`
  - `rerank_policy_json TEXT NOT NULL`
  - `hydrate_policy TEXT NOT NULL`
  - `context_policy_json TEXT NOT NULL`
  - `degrade_policy TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - `status` 只允许 `enabled`、`reserved`、`disabled`。
  - `required_source_version` 必填；第一阶段没有 latest 版本语义。
  - `routes_json` 中的 retriever 只能引用 registry 中存在的 retriever ID。
  - `GRAPH`、`SUMMARY_TREE` route 可存为 reserved，但不能在 enabled Profile 中启用执行。
- indexes:
  - `idx_rag_retrieval_profiles_status` on `(status)`
- notes:
  - 本 feature 不执行检索，只保证配置可保存、读取和校验。

### rag_memory_sources

- change_type: create
- fields:
  - `id INTEGER PRIMARY KEY`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `corpus_id TEXT NOT NULL`
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
  - `corpus_id` 固定为 `memory-card`
  - `scope_type` 只允许 `USER`、`AGENT`、`SPACE`
  - `status` 只允许 `PENDING`、`ACTIVE`、`FAILED`、`DISABLED`、`ARCHIVED`、`DELETED`
- indexes:
  - `idx_rag_memory_sources_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_memory_sources_code_version` on `(source_code, source_version)`
- notes:
  - Memory 治理字段不在本 migration 中创建，后续 `Memory 与 Project Doc 领域字段` feature 通过 alter migration 添加。

### rag_project_doc_sources

- change_type: create
- fields: 与 `rag_memory_sources` 相同
- constraints:
  - unique `(source_code, source_version, scope_type, scope_code)`
  - `corpus_id` 固定为 `project-doc`
- indexes:
  - `idx_rag_project_doc_sources_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_project_doc_sources_code_version` on `(source_code, source_version)`
- notes:
  - Project Doc 的 repo、module、path、doc_type 等领域字段不在本 migration 中创建，后续领域字段 feature 添加。

### rag_project_doc_documents

- change_type: create
- fields:
  - `id INTEGER PRIMARY KEY`
  - `document_code TEXT NOT NULL`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `corpus_id TEXT NOT NULL`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `document_type TEXT NOT NULL`
  - `title TEXT`
  - `path TEXT`
  - `body_hash TEXT`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(document_code, source_code, source_version, scope_type, scope_code)`
  - `corpus_id` 固定为 `project-doc`
- indexes:
  - `idx_rag_project_doc_documents_source` on `(source_code, source_version)`
  - `idx_rag_project_doc_documents_scope` on `(scope_type, scope_code)`
- notes:
  - `memory-card` 第一阶段不建 document 表，binding 中 `document_table` 可为空。

### rag_memory_text_units

- change_type: create
- fields:
  - `id INTEGER PRIMARY KEY`
  - `text_unit_code TEXT NOT NULL UNIQUE`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `document_code TEXT`
  - `corpus_id TEXT NOT NULL`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `unit_type TEXT NOT NULL`
  - `text TEXT NOT NULL`
  - `heading_path TEXT NOT NULL`
  - `ordinal INTEGER NOT NULL`
  - `token_count INTEGER`
  - `content_hash TEXT`
  - `status TEXT NOT NULL`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(source_code, source_version, ordinal)`
  - `corpus_id` 固定为 `memory-card`
  - `unit_type` 只允许 `LEAF`、`SUMMARY`、`SYMBOL`、`NOTE`
  - `status` 只允许 `ACTIVE`、`DISABLED`、`DELETED`
- indexes:
  - `idx_rag_memory_text_units_source` on `(source_code, source_version, ordinal)`
  - `idx_rag_memory_text_units_scope_status` on `(scope_type, scope_code, status)`
- notes:
  - 本 feature 只建表，不实现 split 或 TextUnit 写入。

### rag_project_doc_text_units

- change_type: create
- fields: 与 `rag_memory_text_units` 相同
- constraints:
  - unique `(source_code, source_version, ordinal)`
  - `corpus_id` 固定为 `project-doc`
- indexes:
  - `idx_rag_project_doc_text_units_source` on `(source_code, source_version, ordinal)`
  - `idx_rag_project_doc_text_units_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_project_doc_text_units_document` on `(document_code)`
- notes:
  - heading path 使用 JSON 字符串保存，后续检索可以按 hydrate 需要解析。

### rag_memory_text_units_fts

- change_type: create
- fields:
  - SQLite FTS5 virtual table with `text`
  - unindexed columns: `text_unit_code`、`source_code`、`source_version`、`corpus_id`、`scope_type`、`scope_code`
- constraints:
  - FTS 表只作为索引表，Source/TextUnit 是主数据。
- indexes:
  - FTS5 内部索引
- notes:
  - 如果目标 SQLite 编译缺少 FTS5，repository 初始化应返回明确 storage error，不能静默降级成普通表。

### rag_project_doc_text_units_fts

- change_type: create
- fields: 与 `rag_memory_text_units_fts` 相同
- constraints:
  - FTS 表只作为索引表，Source/TextUnit 是主数据。
- indexes:
  - FTS5 内部索引
- notes:
  - 后续 fulltextIndex stage 负责维护该表内容。

### rag_memory_text_units_vec

- change_type: create
- fields:
  - `id INTEGER PRIMARY KEY`
  - `target_type TEXT NOT NULL`
  - `target_code TEXT NOT NULL`
  - `source_code TEXT NOT NULL`
  - `source_version TEXT NOT NULL`
  - `corpus_id TEXT NOT NULL`
  - `scope_type TEXT NOT NULL`
  - `scope_code TEXT NOT NULL`
  - `embedding_model_code TEXT NOT NULL`
  - `embedding_version TEXT NOT NULL`
  - `embedding_text_hash TEXT NOT NULL`
  - `embedding_dimension INTEGER NOT NULL`
  - `embedding BLOB NOT NULL`
  - `status TEXT NOT NULL`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- constraints:
  - unique `(target_type, target_code, embedding_model_code, embedding_version)`
  - `target_type` 第一阶段只允许 `TEXT_UNIT`
  - `corpus_id` 固定为 `memory-card`
- indexes:
  - `idx_rag_memory_vec_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_memory_vec_source` on `(source_code, source_version)`
  - `idx_rag_memory_vec_model` on `(embedding_model_code, embedding_version)`
- notes:
  - `embedding` 使用 BLOB 保存后端可解码的 f32 向量。第一版向量召回由后端 brute-force cosine 完成，不依赖 SQLite 扩展。

### rag_project_doc_text_units_vec

- change_type: create
- fields: 与 `rag_memory_text_units_vec` 相同
- constraints:
  - unique `(target_type, target_code, embedding_model_code, embedding_version)`
  - `corpus_id` 固定为 `project-doc`
- indexes:
  - `idx_rag_project_doc_vec_scope_status` on `(scope_type, scope_code, status)`
  - `idx_rag_project_doc_vec_source` on `(source_code, source_version)`
  - `idx_rag_project_doc_vec_model` on `(embedding_model_code, embedding_version)`
- notes:
  - 向量写入行为属于后续 ingest/vectorIndex feature。

### rag_indexing_warnings

- change_type: create
- fields:
  - `id INTEGER PRIMARY KEY`
  - `warning_code TEXT NOT NULL`
  - `corpus_id TEXT NOT NULL`
  - `source_code TEXT`
  - `source_version TEXT`
  - `scope_type TEXT`
  - `scope_code TEXT`
  - `stage_id TEXT`
  - `message TEXT NOT NULL`
  - `metadata TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
- constraints:
  - warning 不保存正文、embedding、provider 原始响应或 API key。
- indexes:
  - `idx_rag_indexing_warnings_source` on `(corpus_id, source_code, source_version)`
  - `idx_rag_indexing_warnings_created` on `(created_at DESC)`
- notes:
  - 本 feature 只建最小表壳。真实 warnings 写入与 debug 展示属于后续诊断 feature。

### rag_search_traces

- change_type: create
- fields:
  - `id INTEGER PRIMARY KEY`
  - `trace_code TEXT NOT NULL UNIQUE`
  - `profile_id TEXT NOT NULL`
  - `workspace_code TEXT NOT NULL`
  - `user_code TEXT`
  - `agent_code TEXT`
  - `query_text TEXT NOT NULL`
  - `route_counts_json TEXT NOT NULL`
  - `timings_json TEXT NOT NULL`
  - `warnings_json TEXT NOT NULL`
  - `candidate_refs_json TEXT NOT NULL`
  - `result_count INTEGER NOT NULL`
  - `token_usage_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `expires_at INTEGER NOT NULL`
- constraints:
  - trace 不保存 hydrate 正文、Context Pack body、embedding、provider 原始响应或 API key。
- indexes:
  - `idx_rag_search_traces_profile_created` on `(profile_id, created_at DESC)`
  - `idx_rag_search_traces_workspace_created` on `(workspace_code, created_at DESC)`
  - `idx_rag_search_traces_expires` on `(expires_at)`
- notes:
  - 本 feature 只建表壳。真实 trace 写入、TTL 清理和 debug 查询属于后续诊断 feature。

## Relationships And Cardinality

- 一个 Corpus 对应一个 `rag_corpus_profiles` 记录。
- 一个 Corpus 最多对应一个 `rag_corpus_table_bindings` 记录。
- 一个 Corpus 可以有多个 Ingest Profile 和多个 Retrieval Profile。
- 一个 Source 由 `(source_code, source_version, scope_type, scope_code)` 唯一识别。
- 一个 Source 可以有多个 TextUnit。
- 一个 Project Doc Source 可以有多个 Document；Memory Source 第一阶段不建 Document。
- 一个 TextUnit 可以对应多个 embedding 记录，不同模型或版本通过 `(embedding_model_code, embedding_version)` 区分。
- FTS 和 Vector 表是索引表，不是 Source/TextUnit 主数据。
- warnings 和 trace 是诊断数据，不反向驱动 Source 状态。

## Migration Plan

1. 新增 `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`。
2. 在 `sqlite/migrations.rs` 中新增 `RAG_FOUNDATION_MIGRATION` 常量。
3. 新增 `SqliteRagRepository::connect(database_url)`，连接时执行 RAG migration。
4. migration 使用 `CREATE TABLE IF NOT EXISTS`、`CREATE INDEX IF NOT EXISTS` 和 `INSERT OR IGNORE` seed，保证重复执行幂等。
5. seed `memory-card`、`project-doc`、`code-graph` Corpus Profile。
6. seed 第一阶段最小 Ingest/Retrieval Profile。Profile 可先为后续 ingest/search 提供配置骨架，真实执行由后续 feature 实现。
7. 不执行数据 backfill，因为这是新 schema。
8. rollback 以删除新增 RAG 表为主；已有业务表不受影响。

## Query And Performance Plan

- Profile 查询按 `profile_id` 主键读取。
- Corpus binding 查询按 `corpus_id` 主键读取。
- filterSchema 校验先按 Corpus 读取 profile，再在内存中解析 `filter_schema_json`。
- Source 查询主要使用 `(source_code, source_version)` 和 `(scope_type, scope_code, status)` 索引。
- TextUnit 查询主要使用 `(source_code, source_version, ordinal)` 和 `(scope_type, scope_code, status)` 索引。
- FTS 查询使用 FTS5 内部索引，普通过滤条件仍通过 scope/source 字段约束。
- Vector 查询第一版由后端按 scope/source/model 过滤后，读取 BLOB 并执行 brute-force cosine；后续向量存储替换不改变 application 契约。
- warnings 查询按 source 或 created_at 排序。
- trace 查询按 workspace/profile 和 created_at 排序，TTL 清理按 `expires_at` 索引。

## Data Governance

- 所有主数据表保留 `scope_type` 和 `scope_code`。
- HTTP workspace access guard 不在数据库层完成；数据库层只保存可过滤的 scope 字段。
- 普通检索后续只读取 `status = ACTIVE` 的 Source/TextUnit。
- metadata 只保存低频扩展信息，不能作为未声明过滤字段绕过 filterSchema。
- warnings、trace、Event payload 不保存正文、Context Pack body、embedding、provider 原始响应或 API key。
- `embedding` BLOB 属于索引数据，只能通过 RAG repository 读写，不能通过 debug API 直接返回。
- `rag_search_traces` 的 TTL 策略为 7 天，具体清理任务由后续诊断 feature 实现。

## Test Data And Fixtures

- SQLite in-memory fixture 应能执行 RAG migration 并 seed 三个 Corpus Profile。
- Fixture 应包含 enabled Corpus：`memory-card`、`project-doc`。
- Fixture 应包含 reserved Corpus：`code-graph`。
- Profile fixture 应覆盖 enabled、reserved、disabled 三种状态。
- filterSchema fixture 应覆盖合法字段、未知字段、类型不匹配字段和跨 Corpus 字段。
- Vector fixture 可使用短 BLOB 测试编码/解码边界，不需要真实模型调用。
- FTS fixture 应验证 FTS5 表能创建；如果本地 SQLite 不支持 FTS5，测试应给出明确失败。

## Validation Commands

- `make be-test`: 验证 RAG migration、repository、filterSchema 和 seed 数据测试通过；expect: 后端测试通过。
- `make be-check`: 验证新增 RAG domain/application/persistence 模块可以编译；expect: backend compile check 通过。
- `make be-test`: 验证 SQLite in-memory repository 可重复执行 migration；expect: 重复 connect 不产生重复 seed 或 schema 错误。
