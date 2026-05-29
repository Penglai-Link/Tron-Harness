# Test Report: graph-store-lpg-domain-contract

## Summary

Focused graph tests, guard checks, backend check, and format check passed after worker merge and lead CRUD completion.

## Evidence Rows

- AC-LPG-01 | red: missing LPG imports failed before implementation | green: node and edge CRUD validation tests passed | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-02 | red: missing typed duplicate and missing mappings failed before implementation | green: duplicate and missing record tests passed | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-03 | red: missing endpoint validation failed before implementation | green: endpoint consistency test passed | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-04 | red: missing typed search and traversal contracts failed before implementation | green: structured search and traversal contract tests passed | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-05 | red: guard boundary did not exist before implementation | green: scoped graph/Cargo guard passed with no raw query or raw vector tokens | command_ref: CMD-BE-LPG-GUARD | browser:not applicable | status: done.
- AC-LPG-06 | red: fake store could not observe scoped port calls before implementation | green: fake store port boundary test passed | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-07 | red: forbidden surface guard did not exist before implementation | green: scoped graph/Cargo guard and architecture tests passed | command_ref: CMD-BE-LPG-GUARD | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/application/graph/service_tests.rs`
- `tron-backend/src/application/graph/service_test_support.rs`

## Commands

- `rtk cargo test --manifest-path tron-backend/Cargo.toml graph:: -- --nocapture`: passed with 13 tests.
- `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "knowledge-graph|memory/fragments|inheritable|grafeo|raw_query|cypher|sparql|gql_query|embedding_vector|Vec<f32>" tron-backend/src/domain/graph tron-backend/src/application/graph tron-backend/Cargo.toml; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture'`: passed.
- `rtk make be-check`: passed.
- `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check`: passed.

## Blockers

- None.
