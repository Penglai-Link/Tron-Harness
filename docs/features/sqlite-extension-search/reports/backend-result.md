# Backend Result

## Backend Evidence

| AC | evidence | command_ref | status |
| --- | --- | --- | --- |
| AC-SES-01 | Shared SQLite extension context added; unit tests cover platform mapping and preferred connection preparation without persisted or cached capability state. `make be-check` passed. | CMD-BE-CHECK | passed |
| AC-SES-02 | RAG schema test confirms FTS and fixed 4096 vec sidecar tables are created idempotently, extension capability/vector registry tables are absent, and business text-unit tables no longer carry index payload columns. | CMD-RAG-SCHEMA | passed |
| AC-SES-03 | Existing retriever IDs remain registered; default profile route IDs still return while storage reads through the sidecar contract. | CMD-RAG-SEARCH | passed |
| AC-SES-04 | Default memory and project document full-text routes use `FULLTEXT_SQLITE_FTS5_SIMPLE_V1` and return storage-scored hits. | CMD-RAG-SEARCH | passed |
| AC-SES-05 | Default memory and project document vector routes use `VECTOR_SQLITE_VEC0_COSINE_V1` and return sqlite-vec storage-scored hits. | CMD-RAG-SEARCH | passed |
| AC-SES-06 | Ingest/retry contract writes index payload directly into sidecars; reconnect search verifies business-only text-unit rows remain searchable. | CMD-RAG-SEARCH | passed |
| AC-SES-07 | Sidecar route failure degradation contract still passes while preserving the existing API shape. | CMD-RAG-SEARCH | passed |

## Validation

- `make be-check` passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed, including non-4096 query embedding route degradation to full-text results and mixed ASCII/CJK FTS5 query tokenization.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts --test rag_service_contracts --test rag_search_api --test rag_diagnostics_api --test rag_diagnostics_contracts` passed after switching default retrieval profiles to FTS5/vec0.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_ingest_contracts --test rag_search_contracts --test rag_diagnostics_contracts --test rag_search_api --test rag_diagnostics_api` passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture` now passes the RAG index file split concern but still fails on pre-existing `src/config.rs` line-count threshold.
- `make be-test` fails for the same pre-existing `src/config.rs` architecture threshold after the RAG-focused suites pass.
