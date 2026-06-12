# Backend Task

## Task Rows

- AC-1: 更新内置 corpus 配置、storage binding 解析与 schema 断言。
- AC-2: 更新 SQLite ingest plan，让 `EMBEDDED_SOURCE` 写入 source 字段和 Memory 治理字段。
- AC-3: 更新 search/debug/retry plan，确保无 source table 时使用 text unit 表完成 source 状态、metadata、filter 和 hydrate。
- AC-4: 增加迁移回填旧 `rag_memory_sources` 数据，并删除旧表。
- AC-5: 保持 `project-doc` split 逻辑和现有契约测试。
