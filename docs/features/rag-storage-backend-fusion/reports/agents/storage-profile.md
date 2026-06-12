# Storage Profile Worker Evidence

## Scope

- branch: `feature/xinlangtao/rag-storage-backend-fusion/backend/storage-profile`
- worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-storage-backend-fusion/backend/storage-profile`
- AC: `AC-RSBF-01`, `AC-RSBF-02`
- clicks=n/a, inputs=n/a, network=n/a, feedback=n/a, artifacts=n/a

## RED

- AC-RSBF-01 / AC-RSBF-02 | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_service_contracts -- --nocapture` | result: failed with expected missing storage backend contract before production changes.
- Failure details: `RagStorageBackend` was not exported by `domain::rag`; `RagStorageBinding` had no `backend` field; the fake service-contract binding literal could not set `backend`.
- AC-RSBF-02 note: the same RED test file included the new `skill-memory-project-fusion-v1` assertions, but the run stopped at the AC-RSBF-01 compile gate before profile assertions executed.

## GREEN

- AC-RSBF-01 / AC-RSBF-02 | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_service_contracts -- --nocapture` | result: passed; `rag_foundation_schema` 7/7, `rag_service_contracts` 15/15.
- AC-RSBF-01 | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture` | result: passed; 4/4, including backend-first graph dispatch and existing skill graph behavior.

## VERIFY

- CMD-BE-CHECK | command: `make be-check` | result: passed.
- CMD-FMT | command: `cargo fmt --manifest-path tron-backend/Cargo.toml --check` | result: passed.
- Focused rerun after formatting | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_service_contracts -- --nocapture` | result: passed; 7/7 and 15/15.
- Focused graph rerun after formatting | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture` | result: passed; 4/4.

## Result

- AC-RSBF-01: implemented explicit `SQLITE` / `GRAPH` storage backend parsing and backend-first `GraphBackedRagRepository` dispatch while preserving SQLite and skill graph behavior.
- AC-RSBF-02: seeded `skill-memory-project-fusion-v1` with `skill-graph`, `memory-card`, and `project-doc` routes, RRF, SQLite and graph retrievers, and graph expansion policy.
- Unresolved: none in this worker scope.
