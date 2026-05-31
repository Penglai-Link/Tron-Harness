# Backend Result: RAG 后端底座 - 检索编排与 Context Pack

## Summary

- status: verified
- execution_owner: backend-worker with lead takeover for report and focused hardening
- branch: feature/langtaoxin/rag-search-context-pack/backend/search-context-pack
- result: RAG search API、Retrieval Profile 编译、filter 合并、Memory/Project Doc fulltext/vector 检索、fusion/dedup/rerank、hydrate、Context Pack 和边界回归均已实现并通过验证。

## Evidence Rows

- AC-RSC-01 | red: 新增 HTTP contract 覆盖缺失请求字段、跨 user accessContext、无 workspace access、missing retrieval profile；实现前 `/api/v1/rag/search` 未注册。 | green: `make be-test` 通过 `tests/rag_search_api.rs`，成功请求返回 `{ code: "SUCCESS", message: "...", data: ... }`，非法请求返回稳定 RAG search 错误码。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_http_contract; inputs=authenticated JSON search payloads including malformed body cross-user accessContext unknown workspace and missing retrieval profile; network=POST /api/v1/rag/search; feedback=uniform JSON success and RAG search error envelopes; artifacts=tron-backend/tests/rag_search_api.rs | status: done.
- AC-RSC-02 | red: 新增 Retrieval Profile compiler contract 覆盖缺少固定 sourceVersion、latest、reserved GRAPH route 和非法 context strategy。 | green: `make be-test` 通过 `retrieval_profile_compiler_rejects_invalid_routes_context_and_version`，seeded retrieval profiles 可编译，非法配置被拒绝。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_unit_contract; inputs=valid and invalid retrieval profile configs; network=none_service_layer; feedback=typed RagError assertions; artifacts=tron-backend/tests/rag_search_contracts.rs | status: done.
- AC-RSC-03 | red: 新增 filter contract 检查未知字段、类型错误和 Request filter 放宽 Profile filter 会被拒绝。 | green: `make be-test` 通过 `search_filters_reject_unknown_types_and_prevent_profile_filter_broadening`，同字段取交集，未声明字段返回 `RAG_SEARCH_FILTER_INVALID`。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_unit_contract; inputs=profile filters request filters and unknown filter fields; network=none_service_layer; feedback=normalized intersection and typed filter errors; artifacts=tron-backend/tests/rag_search_contracts.rs | status: done.
- AC-RSC-04 | red: 新增 Memory search integration contract 检查 inactive、wrong scope、wrong version 不能进入结果，fulltext/vector 命中必须统一为 CandidateHit。 | green: `make be-test` 通过 `memory_search_returns_active_scoped_fulltext_vector_hits_and_context_pack`，只返回 active scoped v1 Memory hit，routeIds 包含 fulltext 和 vector。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=memory-card ingest fixtures and search request; network=none_service_layer; feedback=CandidateHit scope version and route assertions; artifacts=tron-backend/tests/rag_search_contracts.rs | status: done.
- AC-RSC-05 | red: 新增 Project Doc search integration contract 检查 hydrate 必须回填 title、path、headingPath，inactive Source 不可见。 | green: `make be-test` 通过 `project_doc_search_hydrates_heading_path_and_document_metadata`，Project Doc hit 返回文档标题、路径和 heading metadata。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_integration_contract; inputs=project-doc markdown ingest fixture and search request; network=none_service_layer; feedback=hydrated document metadata and text unit assertions; artifacts=tron-backend/tests/rag_search_contracts.rs | status: done.
- AC-RSC-06 | red: 新增 fusion/dedup/rerank contract 检查多 route 重复命中不能重复返回、fake rerank 顺序必须稳定、provider 原始响应不能泄漏。 | green: `make be-test` 通过 `fusion_dedup_and_fake_rerank_produce_stable_ordering`，同一 TextUnit 合并 routeIds，fake rerank 调整顺序。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_unit_contract; inputs=fake embedding and fake rerank runtime results; network=none_fake_runtime; feedback=stable ordering and dedup assertions; artifacts=tron-backend/tests/rag_search_contracts.rs | status: done.
- AC-RSC-07 | red: 新增 context assembly contract 检查 `RESULT_LIST`、`TOKEN_BUDGET_PACK`、`MEMORY_CONTEXT_PACK`、`DOC_QA_CONTEXT_PACK` 的响应形态和 tokenBudget 截断。 | green: `make be-test` 通过 `context_assembly_supports_all_required_strategy_shapes`，四种 context strategy 均按限制返回。 | command_ref: CMD-BE-TEST | browser: clicks=none_backend_unit_contract; inputs=context strategies and token budgets; network=none_service_layer; feedback=result list null pack and bounded context pack assertions; artifacts=tron-backend/tests/rag_search_contracts.rs | status: done.
- AC-RSC-08 | red: 新增边界 contract 检查 search 不能引入 debug/retry/trace/SummaryTree/GraphService/MemoryFlow 行为；上一阶段禁止 search route 的断言需要更新为允许 search。 | green: `make be-test` 与 `make be-check` 均通过，`repository_search_boundary_does_not_add_trace_graph_or_memory_flow_behavior` 仍确认 debug/retry/trace/Graph 越界不存在。 | command_ref: CMD-BE-CHECK | browser: clicks=none_backend_regression_contract; inputs=source scans cargo tests and cargo check; network=none_compile_boundary; feedback=architecture boundary and compile checks pass; artifacts=tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_ingest_contracts.rs | status: done.

## Commands

- CMD-BE-TEST | command: `make be-test` | result: passed | notes: backend unit、integration、HTTP、RAG search、RAG ingest、RAG boundary、architecture tests and doc-tests all passed.
- CMD-BE-CHECK | command: `make be-check` | result: passed | notes: `cargo check --manifest-path tron-backend/Cargo.toml` finished successfully.
- CMD-BASE-BE-TEST | command: `make be-test` | result: passed | notes: base 集成分支合并 worker 后完整后端测试通过。
- CMD-BASE-BE-CHECK | command: `make be-check` | result: passed | notes: base 集成分支合并 worker 后 `cargo check --manifest-path tron-backend/Cargo.toml` 通过。
- CMD-DOCS-FINAL | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-search-context-pack --phase final` | result: passed | notes: feature docs final validation passed after status and evidence updates.

## Changed Paths

- `tron-backend/migrations/20260531_0025_add_rag_search_profiles.sql`
- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_filter.rs`
- `tron-backend/src/application/rag/search_profile.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/tests/rag_search_api.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_service_contracts.rs`

## Blockers

- None.
