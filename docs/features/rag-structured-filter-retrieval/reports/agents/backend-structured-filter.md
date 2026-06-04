# Backend Structured Filter Report

## Summary

- branch: `feature/xinlangtao/rag-structured-filter-retrieval/backend/structured-filter`
- worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-structured-filter-retrieval/backend/structured-filter`
- agent_id: `backend`
- subtask_name: `structured-filter`
- status: passed

## Evidence Rows

- AC-RSF-01
  - red: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` failed because `STRUCTURED_FILTER_SQLITE_V1` resolved as `Unsupported` instead of `Enabled`.
  - green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed; compiler accepts the explicit structured route and default structured route id.
  - command_ref: CMD-BE-RAG-SEARCH
  - browser: clicks=not_applicable_backend_only; inputs=contract_tests; network=not_applicable_backend_only; feedback=registry_and_compiler_assertions; artifacts=tron-backend/tests/rag_search_contracts.rs
  - status: passed

- AC-RSF-02
  - red: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` failed because blank structured searches returned `SearchRequestInvalid`.
  - green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed; blank query executes only the structured route and records zero embedding/rerank calls, while profiles without a structured route return `SearchRequestInvalid`.
  - command_ref: CMD-BE-RAG-SEARCH
  - browser: clicks=not_applicable_backend_only; inputs=blank_query_contract; network=not_applicable_backend_only; feedback=route_capture_no_embedding_no_rerank; artifacts=tron-backend/tests/rag_search_contracts.rs
  - status: passed

- AC-RSF-03
  - red: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` failed because SQLite structured-filter profiles were rejected before retrieval.
  - green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed; SQLite structured-filter returns active scoped `v1` rows with merged profile/route/request filters, stable order, positive score, return fields, and limit.
  - command_ref: CMD-BE-RAG-SEARCH
  - browser: clicks=not_applicable_backend_only; inputs=sqlite_structured_filter_fixture; network=not_applicable_backend_only; feedback=active_scope_version_filter_limit_assertions; artifacts=tron-backend/tests/rag_search_contracts.rs
  - status: passed

- AC-RSF-04
  - red: `make be-test TEST=rag_search_contracts` failed at `rag_search_api` because missing query returned HTTP 400 instead of 200 for a structured-filter profile.
  - green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` passed; missing query succeeds for the structured profile and invalid filters return the uniform `RAG_SEARCH_FILTER_INVALID` envelope.
  - command_ref: CMD-BE-RAG-API
  - browser: clicks=not_applicable_backend_only; inputs=http_missing_query_and_invalid_filter_payloads; network=POST_/api/v1/rag/search; feedback=success_and_error_envelopes_asserted; artifacts=tron-backend/tests/rag_search_api.rs
  - status: passed

- AC-RSF-05
  - red: `make be-test TEST=rag_search_contracts` initially failed after RAG suites because `rag_repository/search.rs` reached 602 lines and tripped `rust_coding_spec_architecture`; this was a regression guard, not accepted final behavior.
  - green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts` passed after an additional GraphSingleNode boundary guard confirmed blank structured-filter requests do not execute against graph-backed corpora; `make be-test` passed after trimming the adapter to 590 lines; query-based RAG, graph boundary, skill graph, architecture, and full backend suites passed.
  - command_ref: CMD-BE-TEST
  - browser: clicks=not_applicable_backend_only; inputs=full_backend_suite; network=not_applicable_backend_only; feedback=rag_and_graph_boundary_regressions_green; artifacts=Makefile,cargo_test_output
  - status: passed

## Commands Run

- `make be-test TEST=rag_search_contracts` -> RED, exited 101 at `rag_search_api`; note: Makefile ignores backend `TEST` and ran the full cargo suite until first failure.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` -> RED, 21 passed / 3 failed for missing structured-filter behavior.
- `cargo fmt --manifest-path tron-backend/Cargo.toml` -> passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` -> GREEN, 24 passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` -> GREEN, 6 passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts` -> GREEN, 4 passed after lead review added the SQLite-only GraphSingleNode boundary assertion.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture` -> GREEN, 4 passed.
- `make be-check` -> passed.
- `make be-check` -> passed again after lead review patch.
- `make be-test TEST=rag_search_contracts` -> failed after RAG suites at architecture line-count guard; fixed before final verification.
- `make be-test` -> passed full backend suite.
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-structured-filter-retrieval --phase final` -> failed on lead-owned canonical artifacts: task rows still `ready`, worktree-plan runtime metadata missing, and `reports/backend-result.md` incomplete evidence/blocker markers.

## Changed Paths

- `AGENTS.md`
- `tron-backend/src/application/rag/search_strategy.rs`
- `tron-backend/src/application/rag/strategy_registry.rs`
- `tron-backend/src/application/rag/search_profile.rs`
- `tron-backend/src/application/rag/search_validation.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_ranking.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/adapters/inbound/http/rag/handler.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_search_api.rs`
- `tron-backend/tests/rag_skill_graph_contracts.rs`
- `docs/features/rag-structured-filter-retrieval/reports/agents/backend-structured-filter.md`

## Blockers Or Residual Risks

- No code blocker remains; full backend tests pass.
- Final feature-doc validation remains for the lead integration pass because canonical task/orchestration/backend-result files still need lead-owned status and evidence updates.
- Residual note: backend `make be-test TEST=...` does not filter tests; focused binary runs used direct `cargo test` after the Makefile path exposed the mismatch.
