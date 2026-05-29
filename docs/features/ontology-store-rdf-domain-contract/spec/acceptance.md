# Acceptance: Ontology Store RDF Domain Contract

## Acceptance Index

- AC-ONT-01 [must] [business] Ontology class, property, constraint, and SHACL commands validate typed resource fields; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ONT-02 [must] [business] Ontology service maps duplicate and missing resources to typed ontology errors; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ONT-03 [must] [business] Ontology data uses the same resolved graph scope boundaries as LPG data; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ONT-04 [must] [business] Ontology search uses typed resource, domain, code, label, and target filters; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ONT-05 [must] [security] Internal callers cannot submit raw RDF, SHACL text, Turtle, or SPARQL strings; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ONT-06 [must] [business] OntologyStore port receives resolved scope and typed ontology requests without Grafeo types; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ONT-07 [must] [regression] The feature adds no HTTP route, Grafeo dependency, LPG persistence, or `inheritable` field; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
