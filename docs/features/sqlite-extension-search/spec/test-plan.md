# Test Plan

## Command Index

- CMD-BE-CHECK: `make be-check`
- CMD-BE-TEST: `make be-test`
- CMD-RAG-SEARCH: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts`
- CMD-RAG-SCHEMA: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema`

## Proof Rows

- AC-SES-01 | layer: backend-unit | case: extension context probes built-in and loadable capabilities on SQLite connections | red: capability test fails before shared context exists | green: context reports capability state without panics | command_ref: CMD-BE-TEST.
- AC-SES-02 | layer: backend-schema | case: RAG migration creates FTS and vector sidecar structures idempotently | red: schema test cannot find sidecar tables or registry | green: schema test finds tables after repeated migration | command_ref: CMD-RAG-SCHEMA.
- AC-SES-03 | layer: backend-regression | case: legacy retriever IDs still search through existing behavior | red: legacy search contracts fail or IDs are unsupported | green: legacy contract tests pass | command_ref: CMD-RAG-SEARCH.
- AC-SES-04 | layer: backend-storage | case: FTS5 retriever returns storage-scored full-text hits without LIKE predicate | red: focused test observes unsupported retriever or no FTS-backed hit | green: focused test passes with new retriever | command_ref: CMD-RAG-SEARCH.
- AC-SES-05 | layer: backend-storage | case: sqlite-vec retriever returns nearest vector hits without application-side scan | red: focused test observes unsupported retriever or no vector-backed hit | green: focused test passes with new retriever | command_ref: CMD-RAG-SEARCH.
- AC-SES-06 | layer: backend-storage | case: rebuild indexes existing rows and persisted rows remain indexed | red: rebuild test finds sidecar rows missing | green: sidecar row counts match eligible rows | command_ref: CMD-RAG-SEARCH.
- AC-SES-07 | layer: backend-service | case: missing extension capability reports route unavailable while legacy route remains available | red: route errors break whole profile or API shape | green: route-level warning/degrade behavior is preserved | command_ref: CMD-BE-TEST.
