# RAG 后端底座 - 模板化 SQLite Adapter

## 背景

当前 RAG 已完成 Memory 与 Project Doc 的写入、检索、诊断和重试链路，但 SQLite adapter 中仍存在按 `memory-card`、`project-doc` 分支拼写表名和 SQL 的实现。这个实现可以跑通 MVP，但会让后续“记忆 Source/TextUnit 合表”变成多处硬改。

本阶段先按已确认方案调整底座：不合并记忆表，只把 SQLite adapter 改成由 `rag_corpora.storage_binding_json`、`field_mapping_json`、`filter_schema_json` 和系统默认契约编译 SQL 执行计划。

## 范围

- 支持 `storageBinding.layout`，第一阶段使用 `SPLIT`。
- 新增 SQLite corpus SQL plan，用配置和实际表列决定 source 表、text unit 表、diagnostics 表、状态列、metadata 列、filter 字段归属和 upsert key。
- 写入改为模板化 upsert：Source、TextUnit、warnings 都使用 SQL plan，不再按 corpus 写死 SQL。
- 检索改为 route 独立执行：每条 route 单独查询、打分，再进入 fusion、dedup、rerank、hydrate/context assembly。
- 诊断和重试入口复用同一 SQL plan。
- 暂不做 Memory Source/TextUnit 合表。

## 边界

- `memory-card` 与 `project-doc` 的业务 payload 解析仍保留在 application 层，因为两者业务字段不同。
- `project-doc` 向量失败可降级、`memory-card` 向量失败 fail-fast 的策略不在本阶段改变。
- 表结构不重命名为 `source_status` / `text_unit_status`；本阶段的 SQL plan 先兼容当前 `status` 字段，后续合表时再做字段迁移。

## 验收点

- SQLite RAG repository 的写入、检索、诊断、重试不再包含具体 Corpus 表名。
- Search route 独立查询，FULLTEXT 和 VECTOR route 先各自产生 CandidateHit，再融合去重。
- 现有 RAG 写入、检索、诊断、重试测试保持通过。
