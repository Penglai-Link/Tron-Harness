# Backend Report: RAG Skill Graph Corpus

## Summary

Implemented an additive `skill-graph` RAG corpus backed by the project graph store. Existing `skill` SQLite corpus, memory-card corpus, and project-doc corpus remain in place and pass the existing regression suite. The graph path writes one `RagSkill` node per skill, optionally writes whitelisted related nodes/edges, searches through graph fulltext/vector routes, expands related graph data after rerank/final limit, and routes graph debug/retry without falling into SQLite table plans.

## Evidence Rows

- AC-RSG-01 | red: full regression exposed additive schema/API compatibility gaps during implementation | green: final `make be-test` passes existing skill, memory-card, project-doc, search API, and service contracts | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=Makefile test commands; network=local test process; feedback=tests pass; artifacts=RAG regression tests | status: done.
- AC-RSG-02 | red: schema contract initially rejected the new `skill-graph` seed as an unexpected corpus | green: `rag_foundation_schema` and `rag_skill_graph_contracts` pass with `skill-graph` enabled and no graph text-unit SQLite table | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=migration tests; network=local SQLite and graph storage; feedback=tests pass; artifacts=migration and schema tests | status: done.
- AC-RSG-03 | red: focused skill-graph contract failed before graph write path and payload fixture were complete | green: `skill_graph_ingest_search_expand_debug_and_retry_use_graph_storage_with_fake_models` writes one `RagSkill` node and keeps idempotency | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=fake ingest payload; network=local fake runtime; feedback=graph node assertions pass; artifacts=`rag_skill_graph_contracts.rs` | status: done.
- AC-RSG-04 | red: relatedGraph contract fails on disallowed relationships before whitelist enforcement | green: `skill_graph_related_payload_rejects_disallowed_relationships` passes and valid related node/edge upsert is covered by the integration test | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=relatedGraph JSON payloads; network=local graph store; feedback=whitelist assertions pass; artifacts=related graph contract test | status: done.
- AC-RSG-05 | red: graph route returned `StorageBindingInvalid` while expansion depended on public metadata fields | green: internal scope fields now drive expansion; graph fulltext/vector routes return ranked `CandidateHit` rows | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=graph search request; network=local fake runtime; feedback=search result assertions pass; artifacts=graph search contract | status: done.
- AC-RSG-06 | red: previous dedup identity did not include `sourceCode` and canonical text could collapse cross-corpus/scope hits | green: dedup identity now includes corpus, source, sourceVersion, scope, and target; existing ranking regression suite passes | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=route ranking tests; network=local test process; feedback=dedup regressions pass; artifacts=search ranking code and tests | status: done.
- AC-RSG-07 | red: relation expansion before final candidate selection would expand non-final hits | green: search flow is `rank_route_rows -> fuse_and_dedup -> optional rerank -> final limit -> graph expansion -> assemble_context_pack` | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=search profile with graphExpansionPolicy; network=local graph store; feedback=final hit metadata contains relatedGraph; artifacts=skill-graph contract test | status: done.
- AC-RSG-08 | red: graph layout can hit SQLite `StorageBindingInvalid` if debug/retry are not routed explicitly | green: graph debug returns one ordinal-0 text unit; fulltext retry no-ops successfully; vector retry returns typed unsupported | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=debug and retry requests; network=local graph store; feedback=debug/retry assertions pass; artifacts=diagnostics contract coverage | status: done.
- AC-RSG-09 | red: model-dependent tests would be unsafe if they touched company embedding/rerank providers | green: fake runtime and local temp graph storage cover embedding/rerank-sensitive paths; final verification uses no provider network | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=fake embedding/rerank runtime; network=no external provider calls; feedback=fake call counters pass; artifacts=fake runtime test fixture | status: done.

## Changed Paths

- `tron-backend/migrations/20260604_0031_create_skill_graph_rag_corpus.sql`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_validation.rs`
- `tron-backend/src/application/rag/search_ranking.rs`
- `tron-backend/src/application/rag/search_profile.rs`
- `tron-backend/src/application/rag/search_strategy.rs`
- `tron-backend/src/application/rag/strategy_registry.rs`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest/executor.rs`
- `tron-backend/src/application/rag/service.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs`
- `tron-backend/src/app.rs`
- `tron-backend/tests/rag_skill_graph_contracts.rs`
- `tron-backend/tests/rag_foundation_schema.rs`
- `tron-backend/tests/rag_service_contracts.rs`

## Commands

- `make be-test`: baseline pass before behavior changes; final pass after implementation and formatting.
- `make be-check`: pass after module wiring and final formatting.
- `cargo fmt --manifest-path tron-backend/Cargo.toml -- --check`: pass after rustfmt.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts`: pass, 3 tests.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture backend_source_files_do_not_become_large_implementation_buckets`: pass after adapter/search module split.

## Blockers

- None.
