# Filter Routing Worker Report

## AC-RSBF-03

- Scope: request search filters now match the current route corpus filter schema; profile and route filters remain strict through `merge_search_filters`.
- RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts request_filters_match_each_route_filter_schema -- --nocapture` failed before implementation with `SearchFilterInvalid` on mixed `memoryType`, `documentType`, and `skillName` request filters.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts request_filters_match_each_route_filter_schema -- --nocapture` passed after adding request-schema-aware filter merging.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture` passed, 21 tests.
- VERIFY: `make be-check` passed.
- VERIFY: `cargo fmt --manifest-path tron-backend/Cargo.toml --check` passed.
- Browser evidence: clicks=n/a, inputs=n/a, network=n/a, feedback=n/a, artifacts=n/a; backend-only AC with automated Rust evidence above.

## Notes

- The strict `merge_search_filters` behavior remains covered by `search_filters_reject_unknown_types_and_prevent_profile_filter_broadening`.
- Request filters ignore fields absent from the active corpus schema, while invalid value types for declared fields still return `SearchFilterInvalid`.
