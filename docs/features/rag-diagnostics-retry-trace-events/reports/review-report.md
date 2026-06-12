# Review Report: RAG 后端底座 - 诊断、重试、Trace 与事件

## Summary

- status: verified
- reviewer: lead
- branch: feature/langtaoxin/rag-diagnostics-retry-trace-events/base
- result: 已复核 backend-result、实现边界和命令证据；未发现阻塞问题。

## Evidence Rows

- AC-RDT-01 | red: review checked backend-result RED evidence for missing debug HTTP wiring and request validation. | green: review confirmed `tests/rag_diagnostics_api.rs` covers required fields, workspace access, direct scope and uniform envelope. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=backend-result and debug API tests; network=GET /api/v1/rag/debug/source; feedback=review pass; artifacts=docs/features/rag-diagnostics-retry-trace-events/reports/backend-result.md | status: done.
- AC-RDT-02 | red: review checked backend-result RED evidence for missing read-only diagnostic model. | green: review confirmed debug source/profile excludes embedding BLOB, body and provider raw payload. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=debug service tests and SQLite warning fixture; network=none_service_layer; feedback=review pass; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-03 | red: review checked retry RED evidence for unsupported stage acceptance before implementation. | green: review confirmed retry API accepts only `fulltextIndex` and `vectorIndex`. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=retry API tests; network=POST /api/v1/rag/retry/index; feedback=review pass; artifacts=tron-backend/tests/rag_diagnostics_api.rs | status: done.
- AC-RDT-04 | red: review checked fulltext retry RED evidence for unchanged failed rows. | green: review confirmed SQLite fulltext retry rebuilds `search_text/fulltext_status` from persisted TextUnits. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=retry SQLite tests; network=none_sqlite_integration; feedback=review pass; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-05 | red: review checked vector retry RED evidence for missing fake runtime update and unsafe warning handling. | green: review confirmed vector retry updates embedding fields and writes safe warnings without provider raw response. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=fake embedding runtime success and failure tests; network=none_fake_runtime; feedback=review pass; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-06 | red: review checked trace RED evidence for missing request/result fields and repository write. | green: review confirmed SEARCH_TRACE writes only when enabled, stores lightweight payload and sets 7-day TTL. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=trace enabled and disabled search tests; network=POST /api/v1/rag/search; feedback=review pass; artifacts=tron-backend/tests/rag_diagnostics_api.rs | status: done.
- AC-RDT-07 | red: review checked event RED evidence for missing lifecycle events and rejected oversized stage payload. | green: review confirmed ingest/search/retry/stage/route events use safe fields and stay within EventBus limits. | command_ref: CMD-BE-TEST | browser: clicks=none_lead_review; inputs=recording event sink tests; network=none_event_bus; feedback=review pass; artifacts=tron-backend/tests/rag_diagnostics_contracts.rs | status: done.
- AC-RDT-08 | red: review checked old boundary failures after diagnostics/retry/trace became in-scope. | green: review confirmed updated boundary still excludes SummaryTree, GraphService, GRAPH route, code-graph execution, Profile UI and Memory governance flow. | command_ref: CMD-BE-CHECK | browser: clicks=none_lead_review; inputs=source boundary tests and architecture tests; network=none_compile_boundary; feedback=review pass; artifacts=tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_service_contracts.rs | status: done.

## Commands

- `make be-test` | result: passed.
- `make be-check` | result: passed.
- `cargo fmt --manifest-path tron-backend/Cargo.toml --check` | result: passed.
- `git diff --check` | result: passed.

## Changed Paths

- `docs/features/rag-diagnostics-retry-trace-events/reports/backend-result.md`
- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/*`
- `tron-backend/src/application/rag/*`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/tests/rag_diagnostics_api.rs`
- `tron-backend/tests/rag_diagnostics_contracts.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_service_contracts.rs`

## Residual Risk

- None.

## Blockers

- None.
