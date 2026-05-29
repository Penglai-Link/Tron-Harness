# Test Plan: Embedded Grafeo Adapter Foundation

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user goal continuation
- approved_at: 2026-05-29

Valid statuses: `draft_pending_confirmation`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-GRAFEO-01 | layer: backend | case: embedded catalog and scope bootstrap | red: no production adapter exists to create or bind embedded Grafeo catalog, schema, or named graph | green: contract tests prove one embedded catalog exists, namespaces bind to schemas, and root workspace codes bind to named graphs | command_ref: CMD-BE-GRAFEO-CONTRACT.
- AC-GRAFEO-02 | layer: backend | case: required graph isolation property and timestamp persistence | red: graph writes do not expose stored timestamps or required scope properties through the stable contract | green: adapter tests prove node and edge writes store and read back timestamps and required isolation dimensions through `GraphNode` and `GraphEdge` | command_ref: CMD-BE-GRAFEO-CONTRACT.
- AC-GRAFEO-03 | layer: backend | case: named-graph and lineage isolation | red: graph CRUD, search, or traversal can cross root named-graph boundaries or ignore visible workspace lineage | green: adapter tests prove operations stay inside one named graph and apply the resolved visible workspace filter | command_ref: CMD-BE-GRAFEO-CONTRACT.
- AC-GRAFEO-04 | layer: backend | case: typed ontology persistence on embedded Grafeo | red: ontology operations still rely on fake stores or cannot persist typed resources | green: adapter tests prove typed class, property, domain, range, constraint, and shape persistence and scoped search on embedded Grafeo | command_ref: CMD-BE-GRAFEO-CONTRACT.
- AC-GRAFEO-05 | layer: backend | case: typed error boundary and capability gaps | red: adapter leaks Grafeo types or silently weakens behavior when a capability is missing | green: tests prove capability gaps map to typed unsupported behavior or storage errors and public services remain typed-only | command_ref: CMD-BE-GRAFEO-CONTRACT.
- AC-GRAFEO-06 | layer: regression | case: no graph HTTP surface added | red: adapter work adds public graph routes while wiring runtime services | green: guard finds no `/api/v1/knowledge-graph` or `/api/v1/memory/fragments` route and architecture tests still pass | command_ref: CMD-BE-GRAFEO-GUARD.

## Command Index

- CMD-BE-GRAFEO-CONTRACT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test graph_grafeo_adapter_contract -- --nocapture` | cost: medium | when: red-green-verify | rerun: after Grafeo adapter, graph runtime wiring, graph error taxonomy, or graph config changes | expect: embedded adapter contract tests fail in RED and pass in GREEN.
- CMD-BE-GRAFEO-GUARD | command: `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "/api/v1/knowledge-graph|/api/v1/memory/fragments" tron-backend/src/adapters/inbound tron-backend/src/app.rs; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture'` | cost: medium | when: verify | rerun: after app wiring or route changes | expect: no graph or memory HTTP route is added and architecture tests pass.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after Rust source edits | expect: backend compiles through the project Makefile.
- CMD-BE-TEST | command: `rtk make be-test` | cost: high | when: verify | rerun: after Grafeo adapter or graph contract edits | expect: backend test suite passes with embedded Grafeo dependency and adapter tests enabled.
- CMD-BE-FMT | command: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: verify | rerun: after Rust source edits | expect: adapter and graph files are formatted.

## Evidence Rules

- RED and GREEN evidence for adapter behavior must use the same Grafeo-backed contract test command.
- Adapter evidence must prove real embedded storage behavior rather than fake-store behavior.
- Guard evidence must show no graph or memory HTTP route and architecture tests passing.
- No browser evidence is required because this feature remains backend-internal.

## Regression Scope

- Re-run Grafeo adapter contract tests after any change to `domain/graph`, `application/graph`, `adapters/outbound/persistence`, `config.rs`, or `app.rs`.
- Re-run backend compile checks after changing dependency features or runtime wiring.
- Re-run the HTTP-route guard before starting embedding and hybrid search orchestration.
