# Test Plan

## Command Index

CMD-1: `make be-test`

Cost: high. Required Makefile-first backend validation.

CMD-2: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts rag_ -- --nocapture`

Cost: medium. Focused RAG event contract validation after the Makefile entrypoint is attempted.

CMD-3: `cargo check --manifest-path tron-backend/Cargo.toml`

Cost: medium. Focused compile validation if full test runtime is too high.

## Proof Rows

AC-1: Verify `rag_diagnostics_contracts` event tests capture only `BusRouteClass::Audit` RAG events and no publish failure is surfaced to business results. Command refs: CMD-1, CMD-2.

AC-2: Verify ingest success emits started, only write/index stage events, completed, and no raw payload content. Verify failing ingest emits failed. Command refs: CMD-1, CMD-2.

AC-3: Verify search success emits started, route completed, completed with result counts. Verify skipped route and failed search events. Command refs: CMD-1, CMD-2.

AC-4: Verify retry success emits started, stage completed, completed; retry vector warning emits warning; retry missing units emits failed. Command refs: CMD-1, CMD-2.

AC-5: Verify serialized events omit query text, document text, hit text, embedding text, provider raw payloads, and secret samples. Command refs: CMD-1, CMD-2.
