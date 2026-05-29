# Test Report: graph-scope-and-isolation-kernel

## Summary

Focused graph scope tests, backend compile check, architecture guard, and format check passed after worker integration.

## Evidence Rows

- AC-GSI-01 | red: focused unit suite failed before implementation | green: `graph_scope_maps_namespace_to_schema_and_root_workspace_to_named_graph` passed | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-02 | red: focused unit suite failed before implementation | green: `child_workspace_reads_only_include_root_to_current_visible_lineage` passed | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-03 | red: focused unit suite failed before implementation | green: `write_dimensions_store_current_workspace_while_named_graph_stays_root` passed | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-04 | red: focused unit suite failed before implementation | green: `missing_app_and_user_normalize_to_storage_sentinel_only_in_storage_dimensions` passed | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-05 | red: focused unit suite failed before implementation | green: `caller_supplied_sentinel_and_reserved_storage_properties_are_rejected` passed | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-06 | red: focused unit suite failed before implementation | green: invalid input, archived workspace, empty lineage, and lookup failure tests passed | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | browser:not applicable | status: done.
- AC-GSI-07 | red: forbidden surface guard had no implemented graph scope slice to verify | green: forbidden search, architecture tests, backend check, and format check passed | command_ref: CMD-BE-GRAPH-SCOPE-GUARD | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/application/graph/scope_resolver.rs`
- `tron-backend/src/domain/graph/scope.rs`

## Commands

- `rtk make be-check`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml graph:: -- --nocapture`: passed with 7 tests.
- `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "knowledge-graph|memory/fragments|inheritable|grafeo" tron-backend/src tron-backend/Cargo.toml; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture'`: passed.
- `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check`: passed.

## Blockers

- None.
