# Test Plan: Ontology Store RDF Domain Contract

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user goal continuation
- approved_at: 2026-05-29

Valid statuses: `draft_pending_confirmation`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-ONT-01 | layer: backend | case: typed ontology resource validation | red: graph ontology domain has no typed class, property, constraint, or shape validation | green: tests prove valid ontology commands normalize and invalid resource fields fail | command_ref: CMD-BE-ONTOLOGY-UNIT.
- AC-ONT-02 | layer: backend | case: duplicate and missing ontology resources | red: duplicate or missing ontology outcomes cannot map to typed errors | green: fake store outcomes map to duplicate and missing ontology resource errors | command_ref: CMD-BE-ONTOLOGY-UNIT.
- AC-ONT-03 | layer: backend | case: scope boundary reuse | red: ontology store calls are not scoped like LPG data | green: fake store records show every ontology operation receives `ResolvedGraphScope` | command_ref: CMD-BE-ONTOLOGY-UNIT.
- AC-ONT-04 | layer: backend | case: typed ontology search | red: ontology search lacks typed resource/domain/code/label/target filters | green: tests build typed ontology search queries without raw RDF or SPARQL strings | command_ref: CMD-BE-ONTOLOGY-UNIT.
- AC-ONT-05 | layer: backend | case: raw RDF and SPARQL shield | red: callers can bypass typed contract with raw RDF, SHACL text, Turtle, or SPARQL | green: source guard and tests prove only typed ontology resources are accepted | command_ref: CMD-BE-ONTOLOGY-GUARD.
- AC-ONT-06 | layer: backend | case: OntologyStore scoped port boundary | red: store calls are not scoped or leak Grafeo/RDF library types | green: fake store records show every operation receives `ResolvedGraphScope` plus typed ontology requests | command_ref: CMD-BE-ONTOLOGY-UNIT.
- AC-ONT-07 | layer: regression | case: no forbidden surface | red: implementation adds HTTP, Grafeo, LPG persistence, or inheritable surface | green: guard finds no forbidden route, dependency, raw RDF/SPARQL, or persistence flag | command_ref: CMD-BE-ONTOLOGY-GUARD.

## Command Index

- CMD-BE-ONTOLOGY-UNIT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml graph:: -- --nocapture` | cost: medium | when: red-green-verify | rerun: after graph ontology domain, service, or port changes | expect: graph scope, LPG, and ontology contract tests fail in RED and pass in GREEN.
- CMD-BE-ONTOLOGY-GUARD | command: `rtk proxy bash -lc 'set -euo pipefail; ! rg -n "knowledge-graph|memory/fragments|inheritable|grafeo|raw_rdf|rdf_statement|sparql|turtle|shacl_text|raw_shacl" tron-backend/src/domain/graph tron-backend/src/application/graph tron-backend/Cargo.toml; cargo test --manifest-path tron-backend/Cargo.toml --test rust_coding_spec_architecture'` | cost: medium | when: verify | rerun: after ontology contract or manifest changes | expect: no HTTP graph route, no Grafeo dependency, no raw RDF/SPARQL surface, no raw SHACL text, and architecture tests pass.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after Rust source edits | expect: backend compiles through the project Makefile.
- CMD-BE-FMT | command: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: verify | rerun: after Rust source edits | expect: backend graph source is formatted.

## Evidence Rules

- RED evidence must prove missing ontology behavior with focused graph tests.
- GREEN evidence must use the same focused graph test command.
- Guard evidence must show no HTTP route, no Grafeo dependency, no raw RDF/SPARQL, no raw SHACL text, no `inheritable` field, and architecture tests passing.
- No browser evidence is required because this feature is pure backend and internal Rust only.

## Regression Scope

- Re-run graph tests after any change to ontology contracts or graph service exports.
- Re-run backend check after changing facade exports or service dependencies.
- Re-run guard checks before starting the Grafeo adapter feature.
