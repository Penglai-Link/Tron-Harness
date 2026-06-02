# Test Plan

## Command Index

- CMD-BE-CHECK: `make be-check`
- CMD-BE-TEST: `make be-test`
- CMD-RAG-SEARCH: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts`
- CMD-RAG-SCHEMA: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_foundation_schema`

## Proof Rows

- AC-SES-01 | layer: backend-unit | case: extension context prepares SQLite connections without persisted or cached capability state | red: connection preparation fails before shared context exists | green: context prepares connections without panics | command_ref: CMD-BE-TEST.
- AC-SES-02 | layer: backend-schema | case: RAG migration creates FTS and fixed 4096 vector sidecar structures idempotently while removing main-table index payload columns | red: schema test cannot find sidecar tables, still finds registry/capability tables, or still finds index payload columns on business tables | green: schema test finds only approved sidecar tables and business-only text-unit columns after repeated migration | command_ref: CMD-RAG-SCHEMA.
- AC-SES-03 | layer: backend-regression | case: existing retriever IDs remain supported through sidecar joins | red: existing retriever IDs are unsupported or attempt to read removed main-table index columns | green: existing route contract tests pass through sidecar-backed storage | command_ref: CMD-RAG-SEARCH.
- AC-SES-04 | layer: backend-storage | case: default memory and project document full-text routes return storage-scored FTS5 hits without LIKE predicate | red: seeded retrieval profiles still use LIKE retrievers or no FTS-backed hit | green: focused test passes with FTS5 default routes | command_ref: CMD-RAG-SEARCH.
- AC-SES-05 | layer: backend-storage | case: default memory and project document vector routes return nearest sqlite-vec hits without application-side scan | red: seeded retrieval profiles still use cosine-scan retrievers or no vector-backed hit | green: focused test passes with vec0 default routes | command_ref: CMD-RAG-SEARCH.
- AC-SES-06 | layer: backend-storage | case: ingest/retry writes directly to sidecars and reconnect preserves sidecar-backed search without main-table payload | red: persisted rows require main-table `search_text` or embedding fields to remain searchable | green: search and diagnostics pass after reconnect with business-only text-unit tables | command_ref: CMD-RAG-SEARCH.
- AC-SES-07 | layer: backend-service | case: missing sidecar or non-4096 vector query reports route unavailable without changing API shape | red: route errors break whole profile or API shape | green: route-level warning/degrade behavior is preserved | command_ref: CMD-BE-TEST.
