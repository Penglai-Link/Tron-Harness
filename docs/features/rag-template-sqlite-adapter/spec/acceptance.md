# Acceptance: RAG 模板化 SQLite Adapter

- AC-1: RAG storage binding 能表达 `SPLIT` layout，并保持 Memory 与 Project Doc 当前分表结构可读。
- AC-2: 写入链路根据 SQL plan 执行 Source、TextUnit 和 warning 持久化，不在 repository 中按 corpus 分支写死表名。
- AC-3: 检索链路按 Retrieval Profile route 独立执行，并在应用层完成 fusion、dedup、rerank 和 Context Pack。
- AC-4: 诊断和重试链路复用 SQL plan，不再按 corpus 分支写死 TextUnit 表。
- AC-5: 本阶段不合并 `rag_memory_sources` 与 `rag_memory_text_units`。
