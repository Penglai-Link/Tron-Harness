# Test Plan: Graph Store LPG Domain Contract

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user goal continuation
- approved_at: 2026-05-29

Valid statuses: `draft_pending_confirmation`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-LPG-01 | layer: backend | case: node and edge CRUD command validation | red: graph LPG domain has no typed CRUD command validation | green: tests prove create, update, delete, and get commands validate codes, labels, relationships, text, and properties | command_ref: CMD-BE-LPG-UNIT.
- AC-LPG-02 | layer: backend | case: duplicate and missing graph records | red: duplicate or missing store outcomes cannot map to typed graph errors | green: fake store outcomes map to duplicate node, duplicate edge, missing node, and missing edge errors | command_ref: CMD-BE-LPG-UNIT.
- AC-LPG-03 | layer: backend | case: edge endpoint consistency | red: service can write an edge without checking source and target nodes in scope | green: service checks both endpoints through the same resolved scope before calling create edge | command_ref: CMD-BE-LPG-UNIT.
- AC-LPG-04 | layer: backend | case: typed search and traversal plans | red: search and traversal have no typed plan for combined retrieval dimensions | green: tests build structured, full-text, vector text, label, traversal, limit, and score options without raw query strings | command_ref: CMD-BE-LPG-UNIT.
- AC-LPG-05 | layer: backend | case: raw query shield and vector input boundary | red: caller can bypass typed contract with raw query strings or raw vectors | green: source guard and tests prove only typed clauses and vector text are accepted | command_ref: CMD-BE-LPG-GUARD.
- AC-LPG-06 | layer: backend | case: GraphStore scoped port boundary | red: store calls are not scoped or leak adapter-specific types | green: fake store records show every operation receives `ResolvedGraphScope` plus typed LPG requests only | command_ref: CMD-BE-LPG-UNIT.
- AC-LPG-07 | layer: regression | case: no forbidden surface | red: implementation adds HTTP, Grafeo, RDF, or inheritable surface | green: guard finds no forbidden route, dependency, RDF contract, or persistence flag | command_ref: CMD-BE-LPG-GUARD.

## Command Index

- CMD-BE-LPG-UNIT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml graph:: -- --nocapture` | cost: medium | when: red-green-verify | rerun: after graph LPG domain, service, or port changes | expect: graph scope and LPG contract tests fail in RED and pass in GREEN.
- CMD-BE-LPG-GUARD | command: `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "knowledge-graph|memory/fragments|inheritable|grafeo|raw_query|cypher|sparql|gql_query|embedding_vector|Vec<f32>" tron-backend/src/domain/graph tron-backend/src/application/graph tron-backend/Cargo.toml; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture'` | cost: medium | when: verify | rerun: after graph contract or manifest changes | expect: no HTTP graph route, no Grafeo dependency, no raw query/vector input surface, no RDF leakage, and architecture tests pass.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after Rust source edits | expect: backend compiles through the project Makefile.
- CMD-BE-FMT | command: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: verify | rerun: after Rust source edits | expect: backend graph source is formatted.

## Evidence Rules

- RED evidence must prove missing LPG behavior with focused graph tests.
- GREEN evidence must use the same focused graph test command.
- Guard evidence must show no HTTP route, no Grafeo dependency, no raw query strings, no caller vector input, no RDF contract, no `inheritable` field, and architecture tests passing.
- No browser evidence is required because this feature is pure backend and internal Rust only.

## Regression Scope

- Re-run graph tests after any change to `domain/graph` or `application/graph`.
- Re-run backend check after changing facade exports or service dependencies.
- Re-run guard checks before starting Grafeo adapter and embedding orchestration features.
