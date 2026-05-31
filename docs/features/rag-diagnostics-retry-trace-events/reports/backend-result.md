# Backend Result: RAG 后端底座 - 诊断、重试、Trace 与事件

## Summary

- status: verified
- execution_owner: backend-worker with lead takeover
- branch: feature/langtaoxin/rag-diagnostics-retry-trace-events/backend/diagnostics-retry-trace-events
- result: Debug API、Index Retry API、轻量 Search Trace、RAG EventBus 安全事件、warnings 持久化和回归边界均已实现并通过验证。

## Execution Notes

- 原 backend-worker 停止后未留下磁盘改动；lead 接管 worker worktree 完成 RED -> GREEN -> VERIFY，并保留本报告作为 canonical evidence。
- 本阶段仍未实现 SummaryTree、GraphService、GRAPH route、code-graph execution、Profile UI、Memory governance flow。

## Evidence Rows

- AC-RDT-01 | red: `make be-test` 在只加入诊断 API 合同后失败，编译报缺少 `RagDebugSourceRequest`、`RagDiagnosticsService` 和 debug HTTP wiring。 | green: `make be-test` 通过 `tests/rag_diagnostics_api.rs`，malformed query 返回 `RAG_DEBUG_REQUEST_INVALID`，跨 scope 返回 `RAG_DEBUG_SCOPE_INVALID`，成功响应保持统一 envelope。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_http_contract; inputs=debug source query with missing fields cross-scope and valid SPACE scope; network=GET /api/v1/rag/debug/source; feedback=uniform JSON error and success envelopes; artifacts=tron-backend/tests/rag_diagnostics_api.rs | status: done.
- AC-RDT-02 | red: `make be-test` 在只加入 debug service 合同后失败，缺少 Profile/Source/TextUnit/warning 只读诊断模型。 | green: `make be-test` 通过 `debug_source_returns_warnings_without_embedding_payload`，返回 profile/source/textUnit 摘要和 warnings，响应不包含 embedding BLOB 或 provider payload。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_service_contract; inputs=SQLite persisted source text units and diagnostics warnings; network=none_service_layer; feedback=debug payload excludes forbidden fields; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-03 | red: `make be-test` 在只加入 retry 合同后失败，缺少 `RagRetryIndexRequest`、`RagRetryIndexService` 和 retry stage 校验。 | green: `make be-test` 通过 `retry_api_rejects_unsupported_stage_and_rebuilds_indexes`，`parse` 被拒绝为 `RAG_RETRY_STAGE_UNSUPPORTED`，retry 只执行 index stages。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_http_contract; inputs=retry JSON body with unsupported and fulltextIndex stages; network=POST /api/v1/rag/retry/index; feedback=stable RAG retry error and success stage result; artifacts=tron-backend/tests/rag_diagnostics_api.rs | status: done.
- AC-RDT-04 | red: SQLite retry 合同先失败，失败行的 `search_text/fulltext_status` 无法从已持久化 TextUnit 重建。 | green: `make be-test` 通过 `retry_rebuilds_fulltext_and_vector_from_persisted_text_units`，fulltext retry 将 `search_text` 和 `fulltext_status` 更新为可检索状态。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=failed fulltext rows and sourceVersion v1; network=none_sqlite_integration; feedback=fulltext_status INDEXED and safe stage result; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-05 | red: vector retry 合同先失败，缺少 fake embedding runtime 调用、embedding 字段更新和失败 warning 持久化。 | green: `make be-test` 通过 `retry_rebuilds_fulltext_and_vector_from_persisted_text_units` 与 `vector_retry_failure_persists_safe_warning_without_provider_payload`，成功更新向量字段，失败只保存安全 warning。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=fake embedding success and provider timeout failure; network=none_fake_runtime; feedback=embedding status updated or safe warning persisted; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-06 | red: search trace 合同先失败，`RagSearchRequest` 缺少 `trace` 字段，`RagSearchResult` 缺少 `trace_code`，repository 不能写 `SEARCH_TRACE`。 | green: `make be-test` 通过 `search_trace_writes_lightweight_ttl_row_only_when_enabled` 和 `search_api_writes_trace_only_when_enabled`，只在启用 trace 时写入 query_text、route counts、candidate refs、token usage 和 7 天 TTL。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=trace enabled and disabled search requests; network=POST /api/v1/rag/search; feedback=trace_code returned only when enabled and body/embedding not stored; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs, tron-backend/tests/rag_diagnostics_api.rs | status: done.
- AC-RDT-07 | red: event 合同先失败，ingest/search/retry 生命周期没有发布安全 EventBus 事件，且事件总线失败路径未被证明为 non-blocking；补 ingest event 后 stage payload 字段数超过 EventBus 限制，测试继续失败。 | green: `make be-test` 通过 `rag_ingest_events_use_safe_payload_and_publish_failure_is_non_blocking` 与 `rag_search_events_use_safe_payload_and_publish_failure_is_non_blocking`，RecordingEventSink 观察到 `rag.ingest.started`、`rag.stage.completed`、`rag.ingest.completed`、`rag.search.started`、`rag.route.completed`、`rag.search.completed`，payload 不含正文、embedding 或 secret-like 字段。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_event_contract; inputs=ingest request search request trace request and recording audit sink; network=none_event_bus; feedback=safe lifecycle event names observed and publish failure ignored; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-08 | red: 旧边界测试在本阶段初始实现后失败，因为上一阶段仍禁止 debug/retry/trace 和 EventBus；同时架构测试会拦截超过 600 行的大文件。 | green: `make be-test`、`make be-check`、`cargo fmt --check` 和 `git diff --check` 均通过，边界更新为允许 diagnostics/retry/trace，继续禁止 GraphService、GRAPH route、SummaryTree、code-graph 和 Memory governance flow。 | command_ref: CMD-BE-CHECK | browser: clicks=none_backend_regression_contract; inputs=source scans cargo tests cargo check and formatting checks; network=none_compile_boundary; feedback=architecture boundary and compile checks pass; artifacts=tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_service_contracts.rs, tron-backend/tests/rust_coding_spec_architecture.rs | status: done.

## Commands

- CMD-BE-TEST | command: `make be-test` | result: passed | notes: backend unit、integration、HTTP、RAG diagnostics、RAG retry、RAG trace、RAG event、RAG boundary、architecture tests and doc-tests all passed.
- CMD-BE-CHECK | command: `make be-check` | result: passed | notes: `cargo check --manifest-path tron-backend/Cargo.toml` finished successfully.
- CMD-FMT-CHECK | command: `cargo fmt --manifest-path tron-backend/Cargo.toml --check` | result: passed | notes: Rust formatting check passed.
- CMD-DIFF-CHECK | command: `git diff --check` | result: passed | notes: no whitespace errors.

## Changed Paths

- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/diagnostics.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/retry.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/diagnostics.rs`
- `tron-backend/src/application/rag/events.rs`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/retry.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_observability.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/tests/rag_diagnostics_api.rs`
- `tron-backend/tests/rag_diagnostics_contracts.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_service_contracts.rs`

## Blockers

- None.
