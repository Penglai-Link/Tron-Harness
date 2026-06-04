# Test Plan: RAG Storage Backend Fusion

## Proof Rows

- AC-RSBF-01 | layer: backend | case: explicit storage backend parsing and dispatch | red: seeded corpora lack backend or graph dispatch still keys only on `GRAPH_SINGLE_NODE` | green: tests prove `SQLITE` and `GRAPH` backends parse and GraphBackedRagRepository dispatches by backend | command_ref: CMD-RAG-CONTRACTS.
- AC-RSBF-02 | layer: backend | case: seeded three-corpus fusion profile | red: `skill-memory-project-fusion-v1` is absent | green: profile lists `skill-graph`, `memory-card`, and `project-doc`, with graph and SQLite routes | command_ref: CMD-RAG-SERVICE.
- AC-RSBF-03 | layer: backend | case: request filters match per corpus | red: mixed request filters fail on unknown fields for other corpora | green: mixed `memoryType`, `documentType`, and `skillName` filters return hits from matching corpora and preserve invalid-known-field rejection | command_ref: CMD-RAG-SEARCH.
- AC-RSBF-04 | layer: backend | case: graph semantic projection from RAG query embedding | red: graph vector route calls graph store with `PreparedGraphSearchQuery::plain` | green: fake graph store observes semantic projection and model code for graph vector route | command_ref: CMD-RAG-GRAPH.
- AC-RSBF-05 | layer: backend | case: focused regression set | red: focused commands fail after implementation | green: focused RAG contract commands pass; full Makefile backend status recorded | command_ref: CMD-BE-CHECK.

## Command Index

- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: baseline and final compile | rerun: after Rust source changes | expect: backend compiles.
- CMD-RAG-CONTRACTS | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema --test rag_service_contracts -- --nocapture` | cost: medium | when: storage backend or seeded profile changes | rerun: after migrations/domain storage model changes | expect: storage/profile contracts pass.
- CMD-RAG-SEARCH | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture` | cost: medium | when: search/filter/ranking changes | rerun: after request filter or fusion changes | expect: RAG search contracts pass.
- CMD-RAG-GRAPH | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture` | cost: medium | when: graph-backed RAG changes | rerun: after graph RAG repository or graph vector route changes | expect: skill graph RAG contracts pass.
- CMD-BE-TEST | command: `make be-test` | cost: high | when: final evidence if feasible | rerun: final only | expect: record pass or known unrelated failure with exact failing test.
- CMD-FMT | command: `cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: final | rerun: after source edits | expect: Rust formatting passes.
