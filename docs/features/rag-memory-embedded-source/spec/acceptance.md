# Acceptance

- AC-1: `memory-card` 的 storage binding 使用 `EMBEDDED_SOURCE`，指向 `rag_memory_text_units`，不再暴露 `rag_memory_sources`。
- AC-2: Memory 写入把 Source 字段、Memory 治理字段、全文字段、向量字段写入同一张 `rag_memory_text_units`。
- AC-3: Memory FULLTEXT/VECTOR 检索、debug source、retry index 继续通过模板化 SQLite plan 工作。
- AC-4: 旧 `rag_memory_sources` 数据可迁移回填到 `rag_memory_text_units`，迁移后 RAG 表清单不包含 `rag_memory_sources`。
- AC-5: `project-doc` 保持 Source/TextUnit 分表，现有写入和检索行为不回退。
