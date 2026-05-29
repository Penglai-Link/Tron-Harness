# Test Plan: Embedding And Hybrid Search Orchestration

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user goal continuation
- approved_at: 2026-05-30

Valid statuses: `draft_pending_confirmation`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-EMBED-01 | layer: backend | case: vector-indexed writes derive and persist internal vectors | red: graph writes ignore embedding runtime and persist no vector payload | green: unit and contract tests prove write orchestration calls embedding runtime and Grafeo stores internal vector payloads without vector leakage | command_ref: CMD-BE-GRAPH-EMBED-UNIT; CMD-BE-GRAPH-HYBRID-CONTRACT.
- AC-EMBED-02 | layer: backend | case: chunked vector write path | red: long writes either use one unchunked embedding call or persist partial vector state | green: tests prove chunked writes call `embedding_call_with_chunking` and fail atomically on provider errors | command_ref: CMD-BE-GRAPH-EMBED-UNIT.
- AC-EMBED-03 | layer: backend | case: hybrid search composition | red: vector search cannot combine with structured filters, full-text, and traversal in one scoped request | green: Grafeo-backed tests prove combined clauses produce scoped typed hits with scores | command_ref: CMD-BE-GRAPH-HYBRID-CONTRACT.
- AC-EMBED-04 | layer: backend | case: no silent degrade on embedding failure | red: vector search or vector-indexed writes silently fall back to weaker non-vector behavior | green: service tests prove missing embedding model and provider failure map to typed graph errors and no store mutation | command_ref: CMD-BE-GRAPH-EMBED-UNIT.
- AC-EMBED-05 | layer: regression | case: internal-only graph boundary | red: work adds graph HTTP routes, caller raw vector inputs, or raw Grafeo leakage to public graph contracts | green: guard finds no graph routes or caller raw vector fields and backend architecture tests still pass | command_ref: CMD-BE-GRAPH-EMBED-GUARD.

## Command Index

- CMD-BE-GRAPH-EMBED-UNIT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml application::graph::embedding_tests application::graph::service_tests -- --nocapture` | cost: medium | when: red-green-verify | rerun: after graph domain, graph service, llm runtime integration, or graph error changes | expect: embedding orchestration unit tests fail in RED and pass in GREEN.
- CMD-BE-GRAPH-HYBRID-CONTRACT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test graph_embedding_hybrid_contract -- --nocapture` | cost: medium | when: red-green-verify | rerun: after Grafeo graph adapter or graph embedding persistence changes | expect: Grafeo-backed hybrid contract tests fail in RED and pass in GREEN.
- CMD-BE-GRAPH-EMBED-GUARD | command: `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "/api/v1/knowledge-graph|/api/v1/memory/fragments|Vec<f32>|embedding_vector|query_vector" tron-backend/src/adapters/inbound tron-backend/src/domain/graph tron-backend/src/application/graph; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture -- --nocapture'` | cost: medium | when: verify | rerun: after graph contract or route changes | expect: no graph HTTP route or raw vector caller surface exists and architecture tests pass.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after Rust source edits | expect: backend compiles through the project Makefile.
- CMD-BE-TEST | command: `rtk make be-test` | cost: high | when: verify | rerun: after graph embedding or Grafeo adapter edits | expect: backend test suite passes with embedding orchestration enabled.
- CMD-BE-FMT | command: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: verify | rerun: after Rust source edits | expect: graph embedding files are formatted.

## Evidence Rules

- Embedding orchestration evidence must prove vectors are derived through the real LLM runtime boundary or a fake that implements the same async runtime contract.
- Hybrid search evidence must use the Grafeo-backed contract path and not a fake graph store only.
- Guard evidence must prove no public graph route or caller raw vector field was introduced.
- Browser evidence is not required because this feature remains backend-internal.

## Regression Scope

- Re-run embedding unit tests after any change to `domain/graph`, `application/graph`, `application/llm`, or model runtime resolution.
- Re-run Grafeo hybrid contract tests after any change to `adapters/outbound/persistence/grafeo_*`.
- Re-run guard checks before any later knowledge-graph or memory HTTP surface is introduced.
