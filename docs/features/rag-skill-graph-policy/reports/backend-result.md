# Backend Report: RAG Skill Graph Policy

## Summary

Implemented configurable RAG skill graph write policy. Default `skill-graph` seed no longer whitelists business node kinds or relationships. Explicit profile whitelists still reject out-of-list values. Main-node business fields now come from corpus field mapping, and edge scalar properties are persisted and returned through graph expansion.

## Evidence Rows

- AC-RSGP-01 | red: `make be-test` failed because default seed still had `allowedNodeKinds`, and custom `PROMPT_TEMPLATE` / `USES_PROMPT_TEMPLATE` ingest returned `IngestRequestInvalid` | green: focused `rag_skill_graph_contracts` passes custom kind/relationship ingest, search, and expansion | command_ref: CMD-RSGP-FOCUSED | browser: clicks=none backend-only; inputs=relatedGraph JSON payload; network=local fake runtime and graph store; feedback=focused test passes; artifacts=contract test | status: done.
- AC-RSGP-02 | red: explicit whitelist test defines the rejection boundary | green: configured whitelist test returns `RagError::IngestRequestInvalid` for out-of-list relationship | command_ref: CMD-RSGP-FOCUSED | browser: clicks=none backend-only; inputs=profile config update; network=local SQLite and graph store; feedback=rejection assertion passes; artifacts=contract test | status: done.
- AC-RSGP-03 | red: explorer identified `corpus_policy_defaults` and hardcoded business constants in production graph repository code | green: static scan has no output for removed business constants in production graph repository code | command_ref: CMD-RSGP-STATIC | browser: clicks=none backend-only; inputs=rg scan; network=local filesystem; feedback=no output; artifacts=static scan result | status: done.
- AC-RSGP-04 | red: `make be-test` failed because dynamic `skillAlias` mapping returned no field value | green: focused test writes `skillAlias` through field mapping and search returns it | command_ref: CMD-RSGP-FOCUSED | browser: clicks=none backend-only; inputs=corpus field mapping update and domainFields payload; network=local SQLite and graph store; feedback=field assertion passes; artifacts=contract test | status: done.
- AC-RSGP-05 | red: `make be-test` failed because expanded edge `properties` was null | green: focused tests return edge `required`, `usage`, and `weight` properties | command_ref: CMD-RSGP-FOCUSED | browser: clicks=none backend-only; inputs=edge properties JSON; network=local graph store; feedback=edge assertions pass; artifacts=contract test | status: done.
- AC-RSGP-06 | red: regression would fail if existing corpora or startup wiring broke | green: `make be-check` and full `make be-test` pass | command_ref: CMD-BE-TEST | browser: clicks=none backend-only; inputs=Makefile commands; network=local test process; feedback=all tests pass; artifacts=backend suite | status: done.
- AC-RSGP-07 | red: model-dependent tests would be invalid if they required company provider network | green: focused tests use `FakeRuntime`; full suite passes locally without provider network | command_ref: CMD-RSGP-FOCUSED | browser: clicks=none backend-only; inputs=fake embedding/rerank runtime; network=no external provider calls; feedback=fake runtime assertions pass; artifacts=test fixture | status: done.
- AC-RSGP-07 | red: feature docs final validation failed until required AC/task/orchestration/report templates were completed | green: feature docs final validation passes | command_ref: CMD-DOCS-FINAL | browser: clicks=none backend-only; inputs=feature docs validator; network=local filesystem; feedback=validation OK; artifacts=feature docs | status: done.

## Commands

- `make be-test`: RED failed at `rag_skill_graph_contracts` before implementation; final pass after implementation.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture`: pass, 5 tests.
- `rg -n 'TOOL|CONFIG|VARIABLE|TAG|REQUIRES_TOOL|HAS_CONFIG|USES_VARIABLE|USED_BY_AGENT|HAS_TAG|corpus_policy_defaults' tron-backend/src/adapters/outbound/persistence/rag_graph_repository -S`: pass, no output.
- `make be-check`: pass.
- `make be-test`: pass.
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-skill-graph-policy --phase final`: pass.

## Changed Paths

- `tron-backend/migrations/20260604_0031_create_skill_graph_rag_corpus.sql`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest/executor.rs`
- `tron-backend/src/application/rag/ingest_profile.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/ingest.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/model.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/search.rs`
- `tron-backend/tests/rag_skill_graph_contracts.rs`

## Blockers

- None.
