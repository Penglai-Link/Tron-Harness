# Backend Report: graph-scope-and-isolation-kernel

## Summary

Graph scope kernel implemented and integrated from worker branch `feature/clouds3n/graph-storage-foundation/backend/graph-scope-kernel`. The slice adds pure domain scope types, an application `ScopeResolver`, facade exports, and focused unit tests.

## Evidence Rows

- AC-GSI-01 | red: graph scope module absent and focused tests failed with 0 passed 7 failed | green: resolver maps namespace to schema and root lineage item to named graph | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-02 | red: no visible lineage scope existed | green: visible workspace codes preserve root-to-current path and exclude sibling data | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-03 | red: no storage dimensions separated current workspace writes from root graph | green: storage dimensions use current workspace while named graph stays root workspace | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-04 | red: absent app and user dimensions had no storage-boundary normalization | green: validated command scope keeps optional values and storage dimensions use `__NONE__` | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-05 | red: reserved sentinel and property collision guards were missing | green: explicit `__NONE__` dimensions and required property names return typed errors | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-06 | red: invalid scope cases had no typed graph error taxonomy | green: invalid namespace, invalid workspace, archived workspace, empty lineage, and lookup failure map to typed errors | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-07 | red: guard had no graph implementation to verify | green: guard confirms no HTTP graph route, Grafeo dependency, `inheritable` field, or architecture violation | command_ref: CMD-BE-GRAPH-SCOPE-GUARD | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/domain.rs`
- `tron-backend/src/domain/graph.rs`
- `tron-backend/src/domain/graph/error.rs`
- `tron-backend/src/domain/graph/scope.rs`
- `tron-backend/src/application.rs`
- `tron-backend/src/application/graph.rs`
- `tron-backend/src/application/graph/scope_resolver.rs`

## Commands

- `CMD-BE-GRAPH-SCOPE-UNIT`: passed on base with 7 graph tests.
- `CMD-BE-GRAPH-SCOPE-GUARD`: passed on base with clean forbidden search and 4 architecture tests.
- `CMD-BE-CHECK`: `rtk make be-check` passed on base.
- `CMD-BE-FMT`: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` passed on base.

## Blockers

- None.
