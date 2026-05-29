# Acceptance: Embedded Grafeo Adapter Foundation

## Acceptance Index

- AC-GRAFEO-01 [must] [business] Adapter bootstrap creates or selects one embedded catalog, maps `namespace` to schema, and maps root workspace code to one named graph; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GRAFEO-02 [must] [business] Graph node and edge persistence writes required isolation properties and timestamps and reads them back through the stable typed contract; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GRAFEO-03 [must] [business] Graph CRUD, search, and traversal stay inside one resolved named graph and honor visible workspace lineage; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GRAFEO-04 [must] [business] Ontology CRUD and search persist typed ontology resources in embedded Grafeo with the same scope rules as LPG data; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GRAFEO-05 [must] [security] Caller-facing graph and ontology services still expose only typed contracts, and capability gaps return typed unsupported behavior errors instead of raw Grafeo leakage or silent fallback; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GRAFEO-06 [must] [regression] Backend runtime wiring constructs the embedded Grafeo services without adding graph HTTP routes or frontend-facing graph endpoints; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
