# Backend Result: RAG Multi-Corpus RRF And Topic Strategy

## Summary

- status: done
- result: Implemented profile-driven RRF, canonical dedup, and first-version embedding MMR for multi-corpus RAG search.
- browser: not applicable

## Evidence Rows

- AC-RRT-01 | red: new compiler test failed before policy types existed | green: profile compiler test passes | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RRT-02 | red: multi-corpus RRF test failed before rank fusion was wired | green: RRF ordering passes | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RRT-03 | red: dedup coverage failed before identity and canonical keys were split | green: dedup behavior passes | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RRT-04 | red: MMR test kept a near-duplicate before topic reordering | green: diverse candidate promotion passes | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RRT-05 | red: fallback test covered candidates without vectors | green: fused order fallback passes | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RRT-06 | red: existing RAG regressions were monitored during full run | green: full backend tests pass | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- tron-backend/src/application/rag/search_profile.rs
- tron-backend/src/application/rag/search.rs
- tron-backend/src/application/rag/search_ranking.rs
- tron-backend/src/application/rag/search_observability.rs
- tron-backend/src/application/rag.rs
- tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs
- tron-backend/tests/rag_search_contracts.rs
- docs/features/rag-rrf-topic-strategy

## Commands

- `cargo fmt --manifest-path tron-backend/Cargo.toml`
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts rrf_embedding_mmr_promotes -- --nocapture`
- `make be-test`
- `make be-check`
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-rrf-topic-strategy --phase final`

## Blockers

- None.
