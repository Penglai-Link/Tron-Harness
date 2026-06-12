# RAG Memory Embedded Source

## 背景

上一阶段已经把 SQLite Adapter 改为基于 `rag_corpora.storage_binding_json`、`field_mapping_json`、`filter_schema_json` 编译 SQL 执行计划，Memory 写入和检索已由用户验收通过。

本阶段继续落地“记忆合表”：`memory-card` 不再使用 `rag_memory_sources` + `rag_memory_text_units` 两张业务表，而是把 Source 通用字段、Memory 治理字段、全文字段和向量字段合并到 `rag_memory_text_units`。`project-doc` 仍保持 Source/TextUnit 分表。

## 范围

- `memory-card.storage_binding.layout` 改为 `EMBEDDED_SOURCE`。
- `rag_memory_text_units` 增加 Source 字段：`source_type`、`source_status`、`title`、`source_content_hash`、`source_metadata`、`source_created_at`、`source_updated_at`。
- `rag_memory_text_units` 承载 Memory 治理字段：`memory_type`、`memory_status`、`review_status`、`version`、`superseded_by`、`conflict_group`、`duplicate_group`、`injectable`、`importance_score`、`confidence_score`、`quality_score`、`pin_level`、`expires_at`、`derived_source_type`、`derived_source_code`。
- 旧 `rag_memory_sources` 数据在迁移中回填到 `rag_memory_text_units`，随后删除旧表。
- SQLite Adapter 的写入、检索、debug、retry 继续通过模板化 plan 执行，不按 corpus 写 SQL 分支。

## 边界

- 不合并 `project-doc` 的 Source/TextUnit 表。
- 不实现 code graph corpus。
- 不改变 Memory 的治理状态流转规则；RAG 只存储字段，不执行业务治理逻辑。
- 不改变对外 HTTP 和内部 `RagService` 门面。
