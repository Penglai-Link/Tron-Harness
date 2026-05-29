# Backend Report: graph-store-lpg-domain-contract

## Summary

LPG domain contract implemented and integrated. The slice adds typed node and edge records, CRUD service methods, structured search and traversal query contracts, `GraphStore` port, fake-store tests, and no concrete Grafeo persistence.

## Evidence Rows

- AC-LPG-01 | red: worker RED failed on missing LPG domain and service types | green: CRUD command tests validate codes, labels, relationships, text fields, and typed properties | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-02 | red: duplicate and missing outcomes had no typed graph mapping | green: fake store tests cover duplicate node, duplicate edge, missing node, and missing edge errors | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-03 | red: edge write could bypass endpoint checks | green: service checks source and target nodes through the same resolved scope before edge writes | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-04 | red: search and traversal had no typed plan | green: tests cover structured filters, full-text, vector text, labels, limits, scores, and traversal clauses | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-05 | red: raw query and raw vector boundaries were unguarded | green: graph/Cargo guard finds no raw query or caller vector input surface in graph modules | command_ref: CMD-BE-LPG-GUARD | browser:not applicable | status: done.
- AC-LPG-06 | red: store calls were not scoped by a typed port | green: fake store records prove every operation receives `ResolvedGraphScope` plus typed LPG requests | command_ref: CMD-BE-LPG-UNIT | browser:not applicable | status: done.
- AC-LPG-07 | red: forbidden surface guard was absent | green: guard confirms no HTTP route, Grafeo dependency, RDF contract, or `inheritable` field in this slice | command_ref: CMD-BE-LPG-GUARD | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/domain/graph.rs`
- `tron-backend/src/domain/graph/error.rs`
- `tron-backend/src/domain/graph/lpg.rs`
- `tron-backend/src/application/graph.rs`
- `tron-backend/src/application/graph/service.rs`
- `tron-backend/src/application/graph/service_tests.rs`
- `tron-backend/src/application/graph/service_test_support.rs`

## Commands

- `CMD-BE-LPG-UNIT`: passed with 13 graph tests.
- `CMD-BE-LPG-GUARD`: passed with scoped graph/Cargo search and 4 architecture tests.
- `CMD-BE-CHECK`: `rtk make be-check` passed.
- `CMD-BE-FMT`: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` passed.

## Blockers

- None.
