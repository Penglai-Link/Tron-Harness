# Feature Spec: Ontology Store RDF Domain Contract

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: ontology-store-rdf-domain-contract
- feature_name: Ontology Store RDF Domain Contract
- status: approved_with_defaults
- owner: clouds3n
- source_request: implement PRD feature `Backend Graph Storage - Ontology Store Contract` after graph scope kernel
- main_agent: lead agent on `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/base`
- base_branch: `feature/clouds3n/graph-storage-foundation/base`
- base_commit: current base after LPG contract validation
- delivery_type: new_feature
- dependency_features: graph-scope-and-isolation-kernel complete; graph-store-lpg-domain-contract complete; later embedded-grafeo-adapter-foundation
- dependency_contracts: `ResolvedGraphScope`, `ScopeResolver`, graph error taxonomy, backend Rust facade modules, PRD ontology feature inventory

## Requirement

Tron needs a typed internal ontology contract so backend ontology builders and future graph validation modules can manage RDF-style ontology classes, properties, domains, ranges, constraints, and SHACL shapes without exposing raw RDF or SPARQL strings. The contract must use the same resolved graph scope as LPG data and must stay behind an `OntologyStore` port for the later Grafeo adapter.

## Product Source Slices

- selected_feature: `Feature: Backend Graph Storage - Ontology Store Contract`
- selected_page: `Page: Backend API - Internal Graph Service`
- selected_interaction: `Interaction: Backend API - Manage Ontology Resource`
- selected_rules: `Rule: Backend Graph Storage - Ontology Domain Boundary`; `Rule: Backend Graph Storage - Raw Query Shield`
- selected_exceptions: `Exception: Backend Graph Storage - Ontology Resource Failure`; `Exception: Backend Graph Storage - Invalid Scope`
- selected_acceptance_source: `Acceptance: Backend Graph Storage - Ontology Store Contract`
- selected_workspace: new `docs/features/ontology-store-rdf-domain-contract`
- feature_inventory_summary: ontology contract depends on the completed scope kernel and feeds the embedded Grafeo adapter
- shared_requirement_summary: raw query shielding is enforced here; Grafeo RDF behavior is deferred to adapter implementation
- dependency_map_summary: scope kernel feeds ontology contract; LPG and ontology contracts together feed the Grafeo adapter
- implementation_order_summary: after this contract, the embedded Grafeo adapter can implement both LPG and RDF ports

## Scope Boundary

This feature owns typed ontology domain and application contracts only. It validates ontology classes, properties, domains, ranges, constraints, and SHACL shapes, routes service calls through `OntologyStore`, and proves same-scope behavior with fake-store tests. It does not persist RDF triples in Grafeo, expose SPARQL, implement LPG records, call LLM runtime, add HTTP routes, or add frontend integration.

## Assumptions

- The completed graph scope kernel supplies `ResolvedGraphScope` for ontology operations.
- Ontology resource codes are stable caller-owned identifiers inside one resolved scope.
- First-version ontology domains are `operations`, `development`, `test`, and `design`; namespace-defined extensions are represented as typed custom domain values.
- SHACL shape commands are represented as typed fields and property constraints, not raw Turtle, RDF, or SPARQL payloads.
- Ontology service validation runs before the store port receives commands.
- Duplicate, missing, and inconsistent ontology resources are represented as typed graph errors.

## In Scope

- Ontology domain records for classes, properties, domains, ranges, constraints, and SHACL shapes.
- Typed ontology commands for create, update, delete, get, and search operations.
- `OntologyStore` application port for ontology CRUD and search behavior.
- `OntologyService` application boundary that resolves graph scope, validates commands, prevents raw RDF/SPARQL input, and calls the store port.
- Typed ontology errors for duplicate, missing, invalid, inconsistent, raw query denied, unsupported store behavior, and storage failure.
- Tests using fake scope resolver and fake ontology store implementations.

## Out Of Scope

- Grafeo RDF persistence, triples, named graph storage, indexes, MVCC, or adapter mapping.
- Public SPARQL, Turtle, RDF/XML, raw RDF statement input, or caller-provided query strings.
- LPG node and edge implementation changes.
- LLM runtime embedding calls, vector search, or reranking.
- HTTP controllers, request/response DTOs, frontend pages, or frontend mocks.

## Constraints

- Every ontology store call must carry one `ResolvedGraphScope`.
- Ontology service signatures must not expose Grafeo, RDF library, or SPARQL-specific types.
- Ontology domains must be typed and validated.
- Class, property, constraint, and shape codes must be nonblank and storage-safe.
- Property commands require domain and range references.
- SHACL shapes must be typed data structures, not raw SHACL text.
- Domain modules remain pure and do not import application, adapter, HTTP, SQLite, request client, or event-bus boundaries.

## Feature Model

### Actors

- Internal ontology caller: backend module that requests ontology operations.
- Ontology service: validates commands, resolves scope, and calls `OntologyStore`.
- Scope resolver: existing graph scope dependency that provides `ResolvedGraphScope`.
- Ontology store port: storage abstraction implemented by fake test store now and Grafeo adapter later.

### Core Objects

- `OntologyDomain`: operations, development, test, design, or namespace-defined extension.
- `OntologyClass`: typed class resource.
- `OntologyProperty`: typed property resource with domain and range references.
- `OntologyConstraint`: typed validation rule.
- `OntologyShaclShape`: typed SHACL shape with property constraints.
- `OntologyResource`: enum covering class, property, constraint, and shape resources.
- `OntologySearchQuery`: typed search query over resource type, domain, codes, and labels.
- `OntologyStore`: application port for CRUD and search.
- `OntologyService`: application service that validates, scopes, and calls the port.
- `OntologyError`: typed ontology error taxonomy.

### Rules

- Ontology codes are trimmed and validated before scope resolution reaches the store.
- Ontology domains are typed and cannot be blank.
- Property resources require nonblank domain and range references.
- SHACL shape property constraints require a target property and at least one typed constraint.
- Search queries use typed fields only.
- Raw RDF, Turtle, SHACL text, and SPARQL strings are not accepted from callers.
- Store duplicate or missing responses are mapped without leaking adapter-specific details.

### State Transitions

- caller command -> scope resolver returns `ResolvedGraphScope`
- service validates ontology resource -> invalid commands return `OntologyError`
- valid command -> service calls `OntologyStore` with typed request and resolved scope
- duplicate or missing store result -> service returns typed ontology error
- search query -> service validates typed filters and calls store without raw query strings

## Page And Module Surface

- page_surface: no frontend page and no HTTP route in this version
- internal_surface: Rust modules under `tron-backend/src/domain/graph` and `tron-backend/src/application/graph`
- persistence_surface: `OntologyStore` trait only; future Grafeo adapter implements it
- likely_files_to_add: `tron-backend/src/domain/graph/ontology.rs`; `tron-backend/src/application/graph/ontology.rs`; `tron-backend/src/application/graph/ontology_tests.rs`
- likely_files_to_update: `tron-backend/src/domain/graph.rs`; `tron-backend/src/domain/graph/error.rs`; `tron-backend/src/application/graph.rs`
- test_surface: focused Rust tests under graph ontology application module with fake resolver and fake store

## Repository Findings

- Graph scope kernel already provides scope resolution and required storage dimensions.
- LPG contract already added graph service and store patterns that ontology should mirror without sharing raw query strings.
- Architecture tests enforce pure domain modules, no `mod.rs`, and maximum source file sizes.
- No Grafeo dependency exists in `tron-backend/Cargo.toml`; this feature should not add one.

## Open Decisions

- None. Defaults adopted for execution: synchronous `OntologyStore` port, typed enum domain values, fake-store tests, no raw RDF/SPARQL payloads.
