# Feature Spec: Graph Store LPG Domain Contract

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: graph-store-lpg-domain-contract
- feature_name: Graph Store LPG Domain Contract
- status: approved_with_defaults
- owner: clouds3n
- source_request: implement PRD feature `Backend Graph Storage - LPG Store Contract` after graph scope kernel passes
- main_agent: lead agent on `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/base`
- base_branch: `feature/clouds3n/graph-storage-foundation/base`
- base_commit: current base after graph scope kernel final validation
- delivery_type: new_feature
- dependency_features: graph-scope-and-isolation-kernel complete; later ontology-store-rdf-domain-contract; later embedded-grafeo-adapter-foundation; later embedding-hybrid-search-orchestration
- dependency_contracts: `GraphScopeCommand`, `ResolvedGraphScope`, `ScopeResolver`, `GraphStorageProperty`, `STORAGE_ABSENT_SENTINEL`, backend Rust facade modules, PRD LPG feature inventory

## Requirement

Tron needs a typed internal LPG contract so future knowledge graph, memory, retrieval, app, task, and agent modules can create, update, delete, fetch, search, and traverse business graph records without learning Grafeo APIs or bypassing graph scope rules. This feature defines graph node and edge domain records, dynamic typed properties, query plan types, `GraphStore` application port, and `GraphService` validation flow over an already resolved graph scope.

## Product Source Slices

- selected_feature: `Feature: Backend Graph Storage - LPG Store Contract`
- selected_page: `Page: Backend API - Internal Graph Service`
- selected_interactions: `Interaction: Backend API - Store Graph Record`; `Interaction: Backend API - Search Graph`
- selected_rules: `Rule: Backend Graph Storage - Graph Record Boundary`; `Rule: Backend Graph Storage - Hybrid Query Composition`; `Rule: Backend Graph Storage - Raw Query Shield`
- selected_exceptions: `Exception: Backend Graph Storage - Duplicate Or Missing Graph Record`; `Exception: Backend Graph Storage - Storage Failure`
- selected_acceptance_source: `Acceptance: Backend Graph Storage - LPG Store Contract`
- selected_workspace: new `docs/features/graph-store-lpg-domain-contract`
- feature_inventory_summary: this feature depends on the completed graph scope kernel and defines the stable LPG API before Grafeo adapter work
- shared_requirement_summary: typed graph query contract is built here; Grafeo capability verification and LLM embedding runtime integration remain later feature workspaces
- dependency_map_summary: scope kernel feeds this feature; this feature feeds embedded Grafeo adapter and embedding/hybrid search orchestration
- implementation_order_summary: after this feature, ontology contract can be defined independently, then Grafeo adapter can implement both ports

## Scope Boundary

This feature owns typed LPG domain and application contracts only. It validates graph node and edge commands, guards reserved storage property collisions, defines typed search and traversal plans, routes service calls through `GraphStore`, and uses tests with fake stores. It does not persist data in Grafeo, call LLM runtime embeddings, create RDF ontology records, add HTTP routes, or implement frontend integration.

## Assumptions

- The graph scope kernel is complete and supplies `ResolvedGraphScope` for every store call.
- Node and edge codes are stable caller-owned graph identifiers within one resolved schema and named graph.
- Dynamic properties use a bounded typed enum rather than raw `serde_json::Value` so adapters can map values predictably.
- Labels and relationship names are storage-safe ASCII identifiers in the first version.
- `embedding_text` and `vector_text` are text inputs for later embedding orchestration; callers do not pass raw vectors in this feature.
- Graph service validation should run before the store port receives a command.
- Edge creation and update must validate source and target nodes inside the same resolved scope by querying the store port through the same scope.
- Duplicate and missing record semantics are represented by typed graph errors even while the concrete adapter is deferred.

## In Scope

- LPG domain records: node, edge, label, relationship, typed dynamic properties, full-text text, and embedding text.
- LPG command DTOs for create, update, delete, and get node or edge operations.
- Typed search and traversal query DTOs for structured filters, full-text clauses, vector text clauses, traversal seeds, labels, limits, and scoring options.
- `GraphStore` application port with typed node, edge, search, and traverse operations.
- `GraphService` application boundary that resolves scope, validates commands, prevents reserved property collisions, checks edge endpoints, and calls the store port.
- Typed graph errors for duplicate node, duplicate edge, missing node, missing edge, missing endpoint, invalid command, raw query shield, unsupported store behavior, and storage failure.
- Tests using fake scope resolver and fake store implementations.

## Out Of Scope

- Grafeo dependency declaration, embedded catalog/schema/named-graph setup, indexes, MVCC, persistence, or adapter mapping.
- LLM runtime embedding calls, chunking, vector generation, vector storage, or reranking.
- RDF ontology classes, properties, domains, ranges, constraints, or SHACL shapes.
- HTTP controllers, request/response DTOs, frontend pages, or frontend mocks.
- Public raw query languages such as GQL, Cypher, SPARQL, or Grafeo-specific query strings.
- Caller-supplied embedding vectors as normal write or search input.
- Cross-root graph joins or multi-graph query execution.

## Constraints

- All graph store calls must carry one `ResolvedGraphScope`.
- The service must not expose Grafeo types in public domain or application signatures.
- Business dynamic properties cannot use required storage property names from the scope kernel.
- Nodes require at least one label; edges require nonblank source, target, and relationship values.
- Search queries require at least one structured, full-text, vector text, traversal, or label clause.
- Traversal queries require at least one start node and a positive depth limit.
- Vector search is represented by `vector_text` only; vector materialization is deferred.
- Domain modules remain pure and do not import application, adapter, HTTP, SQLite, request client, or event-bus boundaries.

## Feature Model

### Actors

- Internal graph caller: backend module that requests LPG operations.
- Graph service: validates commands, resolves scope, and calls `GraphStore`.
- Scope resolver: existing graph scope dependency that provides `ResolvedGraphScope`.
- Graph store port: storage abstraction implemented by a fake test store now and Grafeo adapter later.

### Core Objects

- `GraphNode`: scoped node record with code, labels, properties, optional full-text, optional embedding text, and timestamps.
- `GraphEdge`: scoped edge record with code, source node code, target node code, relationship, properties, optional full-text, optional embedding text, and timestamps.
- `GraphPropertyValue`: typed dynamic property value.
- `GraphProperties`: ordered map of dynamic property names to typed values.
- `GraphNodeCommand` and `GraphEdgeCommand`: validated write commands.
- `GraphSearchQuery`: structured search plan with labels, filters, full-text text, vector text, traversal clause, limit, and score options.
- `GraphTraverseQuery`: traversal plan with start nodes, relationship filter, direction, depth, and limit.
- `GraphStore`: application port for node, edge, search, and traversal behavior.
- `GraphService`: application service that owns validation and port orchestration.
- `GraphError`: typed LPG error taxonomy.

### Rules

- Graph record code, label, relationship, and property names are trimmed before validation.
- Graph codes must be nonblank and storage-safe.
- Label and relationship identifiers must be storage-safe.
- Dynamic property names must be nonblank and cannot collide with graph scope reserved storage properties.
- Edge endpoints must exist in the same resolved scope before the edge write reaches `GraphStore::create_edge`.
- Search and traversal requests carry typed clauses only, never raw backend query strings.
- Vector text is accepted as text, not as caller-supplied vectors.
- Store port errors are mapped without leaking adapter-specific details.

### State Transitions

- caller command -> scope resolver returns `ResolvedGraphScope`
- service validates record boundary -> invalid commands return `GraphError`
- edge command -> service checks source and target node existence in the same scope
- valid command -> service calls `GraphStore` with typed request and resolved scope
- store duplicate or missing response -> service returns typed graph duplicate or missing error
- search or traversal query -> service validates typed plan and calls store without raw query strings

## Page And Module Surface

- page_surface: no frontend page and no HTTP route in this version
- internal_surface: Rust modules under `tron-backend/src/domain/graph` and `tron-backend/src/application/graph`
- persistence_surface: `GraphStore` trait only; future Grafeo adapter implements it
- likely_files_to_add: `tron-backend/src/domain/graph/lpg.rs`; `tron-backend/src/application/graph/port.rs`; `tron-backend/src/application/graph/service.rs`
- likely_files_to_update: `tron-backend/src/domain/graph.rs`; `tron-backend/src/domain/graph/error.rs`; `tron-backend/src/application/graph.rs`
- test_surface: focused Rust tests under graph service module with fake resolver and fake store

## Repository Findings

- The completed scope kernel already exports `GraphScopeCommand`, `ResolvedGraphScope`, `GraphStorageProperty`, and `ScopeResolver`.
- Existing backend services keep application ports and service logic in `application/<context>/service.rs` or submodules.
- Current backend services are synchronous, so the first `GraphStore` trait can be synchronous until embedding or external adapter behavior requires async.
- Architecture tests enforce no `mod.rs`, small facade modules, and pure domain boundaries.
- No Grafeo dependency exists in `tron-backend/Cargo.toml`; this feature should not add one.
- `LlmRuntimeService` exists but is intentionally unused in this selected feature.

## Open Decisions

- None. Defaults adopted for execution: synchronous `GraphStore` port, typed property enum, fake-store tests, and vector text fields without raw vector inputs.
