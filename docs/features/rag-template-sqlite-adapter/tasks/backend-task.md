# Backend Task: RAG 模板化 SQLite Adapter

- phase: tdd
- execution_owner: lead
- runtime_agent_type: not_spawned
- reason: 本次改动集中在同一组 RAG adapter/search 文件，拆分 worker 会增加共享文件冲突。
- ac_rows: AC-1, AC-2, AC-3, AC-4, AC-5
- write_scope: `tron-backend/src/domain/rag`, `tron-backend/src/application/rag`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository`, `tron-backend/migrations`, `tron-backend/tests/rag_*`
