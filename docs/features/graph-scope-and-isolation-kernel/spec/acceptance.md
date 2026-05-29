# Acceptance: Graph Scope And Isolation Kernel

## Acceptance Index

- AC-GSI-01 [must] [business] Scope resolution maps namespace to schema and root workspace to named graph; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GSI-02 [must] [business] Child workspace reads include only root-to-current visible workspace lineage; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GSI-03 [must] [business] Writes store the current workspace code while preserving root graph isolation; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GSI-04 [must] [business] Missing app and user dimensions normalize to `__NONE__` only in storage dimensions; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GSI-05 [must] [security] Caller-supplied sentinel values and reserved storage property names are rejected; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GSI-06 [must] [business] Invalid namespace, invalid workspace, archived workspace, and empty lineage return typed graph scope errors; proof: @ref:spec/test-plan.md#proof-rows.
- AC-GSI-07 [must] [regression] The feature adds no HTTP route, Grafeo dependency, or `inheritable` persistence field; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
