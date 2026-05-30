# Backend Result: RAG 后端底座 - 同步写入 DAG

## Summary

- status: verified
- execution_owner: backend-worker with lead takeover for architecture split
- branch: feature/langtaoxin/rag-ingest-dag/backend/ingest-dag
- result: RAG ingest API、同步 DAG 编译、Memory/Project Doc 写入、向量降级、幂等写入和边界回归均已实现并通过验证。

## Evidence Rows

- AC-RID-01 | red: 新增 HTTP contract 覆盖缺失请求字段、未认证请求、无工作区访问和跨 scope 写入；实现前 `/api/v1/rag/ingest` 未注册。 | green: `make be-test` 通过 `tests/rag_ingest_api.rs`，成功请求返回 `{ code: "SUCCESS", message: "ok", data: ... }`，非法请求返回稳定 RAG 错误码。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_http_contract; inputs=authenticated JSON ingest payloads including missing agentCode and cross-scope requests; network=POST /api/v1/rag/ingest; feedback=uniform JSON success and RAG error envelopes; artifacts=tron-backend/tests/rag_ingest_api.rs | status: done.
- AC-RID-02 | red: 新增 DAG compiler contract 覆盖 invalid corpus、cycle、missing stage、unregistered strategy、Artifact 不兼容和缺 persist stage。 | green: `make be-test` 通过 `ingest_profile_compiler_rejects_invalid_dag_contracts`，seeded ingest profiles 可编译，非法 DAG 被拒绝。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_unit_contract; inputs=valid and invalid ingest profile configs; network=none_service_layer; feedback=typed RagError assertions; artifacts=tron-backend/tests/rag_ingest_contracts.rs | status: done.
- AC-RID-03 | red: 新增 Memory 写入 contract 检查 Source、TextUnit、fulltext 字段和 vector 字段，未实现时没有可观察写入。 | green: `make be-test` 通过 `memory_plain_text_ingest_writes_source_text_unit_fulltext_and_vector_fields`，SQLite 中 Source 为 `ACTIVE`，TextUnit 为 `INDEXED`。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=memory-card plain text ingest request; network=none_service_layer; feedback=SQLite row assertions for Source and TextUnit; artifacts=tron-backend/tests/rag_ingest_contracts.rs | status: done.
- AC-RID-04 | red: 新增 Project Doc Markdown contract 检查文档字段和 heading split，未实现时无 Project Doc 落库与切分。 | green: `make be-test` 通过 `project_doc_markdown_ingest_persists_document_fields_and_heading_split_units`，Project Doc Source 字段和 heading-based TextUnit 均写入。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=project-doc Markdown ingest request with heading text; network=none_service_layer; feedback=SQLite source field and heading split assertions; artifacts=tron-backend/tests/rag_ingest_contracts.rs | status: done.
- AC-RID-05 | red: 新增幂等 contract 检查同一 `sourceCode + sourceVersion + scope` 重试不能重复，多版本必须共存。 | green: `make be-test` 通过 `same_source_version_is_idempotent_and_multiple_versions_coexist`，upsert 保持行数稳定并允许新版本并存。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=repeated v1 ingest and separate v2 ingest; network=none_service_layer; feedback=SQLite source/text-unit count assertions; artifacts=tron-backend/tests/rag_ingest_contracts.rs | status: done.
- AC-RID-06 | red: 新增 fake embedding failure contract 检查 Memory fail-fast 与 Project Doc warning degrade。 | green: `make be-test` 通过 `embedding_failure_degrades_project_doc_but_fails_memory`，Project Doc 保持 `ACTIVE` 并写入 `rag_diagnostics` warning，Memory 返回 `RAG_EMBEDDING_FAILED` 并标记 `FAILED`。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_unit_contract; inputs=fake embedding runtime failure for memory-card and project-doc; network=none_fake_runtime; feedback=warning diagnostics and failed memory status assertions; artifacts=tron-backend/tests/rag_ingest_contracts.rs | status: done.
- AC-RID-07 | red: lead 接手时 `make be-test` 失败在 `rust_coding_spec_architecture`，指出 `service.rs` 1359 行、`rag_repository.rs` 696 行；边界测试也禁止 search/debug/retry/SummaryTree/Graph/MemoryFlow。 | green: 拆分为 `ingest.rs`、`ingest_profile.rs`、`ingest_payload.rs` 和 `rag_repository/ingest.rs` 后，`make be-test` 与 `make be-check` 均通过，架构守卫和边界测试均通过。 | command_ref: CMD-BE-CHECK | browser: clicks=none_backend_regression_contract; inputs=source scans and cargo checks; network=none_compile_boundary; feedback=architecture and boundary tests pass; artifacts=tron-backend/tests/rust_coding_spec_architecture.rs, tron-backend/tests/rag_ingest_contracts.rs | status: done.

## Commands

- CMD-BE-TEST | command: `make be-test` | result: passed | notes: backend unit、integration、HTTP、RAG ingest、RAG boundary、architecture tests and doc-tests all passed.
- CMD-BE-CHECK | command: `make be-check` | result: passed | notes: `cargo check --manifest-path tron-backend/Cargo.toml` finished successfully.

## Changed Paths

- `tron-backend/src/adapters/inbound/http/rag.rs`
- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/src/adapters/inbound/http.rs`
- `tron-backend/src/app.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest_profile.rs`
- `tron-backend/src/application/rag/ingest_payload.rs`
- `tron-backend/src/application/rag/service.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/migrations/20260531_0024_add_rag_ingest_profiles.sql`
- `tron-backend/tests/rag_ingest_api.rs`
- `tron-backend/tests/rag_ingest_contracts.rs`
- `tron-backend/tests/rag_service_contracts.rs`

## Blockers

- None.
