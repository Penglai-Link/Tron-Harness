# Test Plan: RAG Structured Filter Retrieval

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-06-04

## Proof Rows

- AC-RSF-01 | layer: backend-unit | case: profile compiler and strategy registry accept `STRUCTURED_FILTER_SQLITE_V1` and expose the structured route kind | red: compiler rejects the new retriever or maps it to full-text/vector behavior | green: profile route compiles with the new retriever and strategy kind is structured-filter | command_ref: CMD-BE-RAG-SEARCH.
- AC-RSF-02 | layer: backend-service | case: blank query runs only structured-filter routes and rejects profiles without one | red: blank query is rejected globally, calls embedding/rerank, or full-text/vector routes run | green: blank query returns structured results, records no embedding/rerank calls, and no-structured-route profile returns `RAG_SEARCH_REQUEST_INVALID` | command_ref: CMD-BE-RAG-SEARCH.
- AC-RSF-03 | layer: backend-integration | case: SQLite structured-filter route applies active status, scope, sourceVersion, merged filters, positive score, stable order, return fields, and limit | red: rows are dropped by zero score, filters are ignored, or inactive/wrong-scope rows leak | green: only matching active scoped rows return in stable order with requested fields | command_ref: CMD-BE-RAG-SEARCH.
- AC-RSF-04 | layer: backend-api | case: HTTP search accepts missing query for structured-filter profile and keeps filter errors stable | red: handler rejects missing query before service validation or returns non-uniform errors | green: missing-query HTTP request succeeds for structured profile and invalid filters return `RAG_SEARCH_FILTER_INVALID` envelope | command_ref: CMD-BE-RAG-API.
- AC-RSF-05 | layer: backend-regression | case: existing query-based RAG search and graph boundary behavior stay stable | red: existing full-text/vector/context/graph tests fail after route gating changes | green: focused RAG contracts and full backend tests pass | command_ref: CMD-BE-TEST.

## Command Index

- CMD-BE-RAG-SEARCH | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` | cost: medium | when: red, green, verify | rerun: after application search, strategy, profile, ranking, or SQLite search adapter changes | expect: focused RAG search contracts pass.
- CMD-BE-RAG-API | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` | cost: medium | when: red, green, verify | rerun: after HTTP request, handler, response, or service validation changes | expect: focused RAG search API contracts pass.
- CMD-BE-RAG-GRAPH | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts` | cost: medium | when: verify | rerun: after graph-backed RAG boundary changes | expect: skill graph contracts pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring changes | expect: backend compile check passes.
- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after all implementation changes before final evidence | expect: backend tests pass.
- CMD-DOCS-FINAL | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-structured-filter-retrieval --phase final` | cost: fast | when: verify | rerun: after feature docs, tasks, orchestration, or reports change | expect: final docs validation passes.

## Evidence Rules

- RED evidence must be from focused tests that fail for the expected missing structured-filter behavior.
- GREEN evidence must rerun the same focused command after production changes.
- Browser evidence is not required because this is backend-only; report rows still use explicit `browser:` fields with non-applicable tokens.
