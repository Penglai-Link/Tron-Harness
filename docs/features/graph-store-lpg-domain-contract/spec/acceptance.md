# Acceptance: Graph Store LPG Domain Contract

## Acceptance Index

- AC-LPG-01 [must] [business] Node and edge CRUD commands validate stable codes, labels, relationships, and typed properties; proof: @ref:spec/test-plan.md#proof-rows.
- AC-LPG-02 [must] [business] Graph service maps duplicate and missing node or edge outcomes to typed graph errors; proof: @ref:spec/test-plan.md#proof-rows.
- AC-LPG-03 [must] [business] Edge writes require source and target nodes in the same resolved scope; proof: @ref:spec/test-plan.md#proof-rows.
- AC-LPG-04 [must] [business] Search and traversal contracts represent structured filters, full-text, vector text, labels, limits, and traversal clauses; proof: @ref:spec/test-plan.md#proof-rows.
- AC-LPG-05 [must] [security] Internal callers cannot use raw graph query strings or caller-supplied vectors; proof: @ref:spec/test-plan.md#proof-rows.
- AC-LPG-06 [must] [business] GraphStore port receives resolved scope and typed LPG requests without Grafeo types; proof: @ref:spec/test-plan.md#proof-rows.
- AC-LPG-07 [must] [regression] The feature adds no HTTP route, Grafeo dependency, RDF contract, or `inheritable` field; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
