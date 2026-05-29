# Test Plan: Graph Scope And Isolation Kernel

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user goal continuation
- approved_at: 2026-05-29

Valid statuses: `draft_pending_confirmation`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-GSI-01 | layer: backend | case: schema and named graph mapping | red: resolving graph scope has no graph module or cannot derive schema and named graph | green: resolver returns schema from namespace and named graph from root workspace code | command_ref: CMD-BE-GRAPH-SCOPE-UNIT.
- AC-GSI-02 | layer: backend | case: lineage read visibility | red: child workspace scope cannot expose root-to-current path without siblings | green: visible workspace codes preserve workspace path order from root to current | command_ref: CMD-BE-GRAPH-SCOPE-UNIT.
- AC-GSI-03 | layer: backend | case: write workspace dimension | red: storage dimensions cannot distinguish current workspace writes from root named graph | green: storage dimensions use current workspace code while named graph remains root workspace code | command_ref: CMD-BE-GRAPH-SCOPE-UNIT.
- AC-GSI-04 | layer: backend | case: sentinel normalization | red: absent app or user dimensions remain optional or null-equivalent in storage payloads | green: storage dimensions normalize absent app and user values to `__NONE__` only after validation | command_ref: CMD-BE-GRAPH-SCOPE-UNIT.
- AC-GSI-05 | layer: backend | case: reserved values and properties | red: caller can pass `__NONE__` or overwrite required storage property names | green: validation rejects explicit sentinel values and required property collisions | command_ref: CMD-BE-GRAPH-SCOPE-UNIT.
- AC-GSI-06 | layer: backend | case: invalid scope taxonomy | red: invalid namespace, invalid workspace, archived workspace, or empty lineage cannot be distinguished | green: resolver returns typed graph scope errors before any store adapter call | command_ref: CMD-BE-GRAPH-SCOPE-UNIT.
- AC-GSI-07 | layer: regression | case: no forbidden surface | red: implementation adds HTTP routes, Grafeo dependency, `mod.rs`, or `inheritable` graph field | green: search and architecture checks find no forbidden graph surface or persistence flag | command_ref: CMD-BE-GRAPH-SCOPE-GUARD.

## Command Index

- CMD-BE-GRAPH-SCOPE-UNIT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml graph:: -- --nocapture` | cost: medium | when: red-green-verify | rerun: after graph scope domain or resolver changes | expect: focused graph scope tests fail in RED and pass in GREEN.
- CMD-BE-GRAPH-SCOPE-GUARD | command: `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "knowledge-graph|memory/fragments|inheritable|grafeo" tron-backend/src tron-backend/Cargo.toml; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture'` | cost: medium | when: verify | rerun: after backend graph module or manifest changes | expect: no HTTP graph route, no Grafeo dependency, no inheritable field, and architecture tests pass.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after Rust source edits | expect: backend compiles through the project Makefile.
- CMD-BE-FMT | command: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: verify | rerun: after Rust source edits | expect: backend graph source is formatted.

## Evidence Rules

- RED evidence must come from the focused graph scope command and must fail for missing behavior, not syntax or dependency noise.
- GREEN evidence must use the same focused graph scope tests that produced RED.
- Guard evidence must show no HTTP graph route, no Grafeo dependency, no `inheritable` persistence field, and architecture tests passing.
- No browser evidence is required because the feature is pure backend and internal Rust only.

## Regression Scope

- Re-run graph scope unit tests after changing scope validation, workspace lineage mapping, sentinel normalization, or reserved property names.
- Re-run backend check after changing facade exports or application service modules.
- Re-run guard checks before integrating any later LPG, ontology, Grafeo, or embedding feature that consumes this scope kernel.
