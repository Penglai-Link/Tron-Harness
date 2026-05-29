# Feature Spec: Embedded Grafeo Adapter Foundation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: embedded-grafeo-adapter-foundation
- feature_name: Embedded Grafeo Adapter Foundation
- status: approved_with_defaults
- owner: clouds3n
- source_request: implement PRD feature `Backend Graph Storage - Embedded Grafeo Adapter` after scope, LPG contract, and ontology contract stabilization
- main_agent: lead agent on `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/base`
- base_branch: `feature/clouds3n/graph-storage-foundation/base`
- base_commit: `c98b0bb`
- delivery_type: new_feature
- dependency_features: graph-scope-and-isolation-kernel complete; graph-store-lpg-domain-contract complete; ontology-store-rdf-domain-contract complete; later embedding-hybrid-search-orchestration
- dependency_contracts: `GraphStore`; `OntologyStore`; `GraphService`; `OntologyService`; `ResolvedGraphScope`; `GraphStorageProperty`; graph and ontology error taxonomies; backend runtime wiring in `tron-backend/src/app.rs`; PRD graph storage feature inventory

## Requirement

Tron needs one production outbound graph adapter that actually persists scoped LPG graph records and typed ontology resources in embedded Grafeo. The adapter must use one embedded catalog, map `namespace` to schema, map root workspace code to named graph, preserve current-workspace lineage isolation with required storage properties, and keep all Grafeo details behind the existing typed `GraphStore` and `OntologyStore` ports.

## Product Source Slices

- selected_feature: `Feature: Backend Graph Storage - Embedded Grafeo Adapter`
- selected_page: `Page: Backend API - Internal Graph Service`
- selected_interactions: `Interaction: Backend API - Store Graph Record`; `Interaction: Backend API - Search Graph`; `Interaction: Backend API - Manage Ontology Resource`
- selected_rules: `Rule: Backend Graph Storage - Catalog Schema Graph Mapping`; `Rule: Backend Graph Storage - Required Grafeo Properties`; `Rule: Backend Graph Storage - Raw Query Shield`
- selected_exceptions: `Exception: Backend Graph Storage - Grafeo Capability Gap`; `Exception: Backend Graph Storage - Storage Failure`
- selected_acceptance_source: `Acceptance: Backend Graph Storage - Embedded Grafeo Adapter`
- selected_workspace: new `docs/features/embedded-grafeo-adapter-foundation`
- feature_inventory_summary: this feature turns the existing graph ports into a real embedded graph database implementation
- shared_requirement_summary: scope kernel still owns namespace and lineage resolution; LPG and ontology contracts still own typed caller contracts; this feature owns only concrete Grafeo persistence and runtime wiring
- dependency_map_summary: scope kernel feeds LPG and ontology contracts; those contracts feed this Grafeo adapter; embedding and hybrid search orchestration depends on this adapter
- implementation_order_summary: complete scope and typed contracts first, then embed Grafeo behind the ports, then build embedding and hybrid orchestration on proven adapter capabilities

## Scope Boundary

This feature owns the backend outbound persistence adapter and internal runtime wiring for embedded Grafeo. It adds the Grafeo dependency, bootstraps one local embedded catalog, creates or selects namespace schemas and root-workspace named graphs, persists LPG node and edge data behind `GraphStore`, persists typed ontology resources behind `OntologyStore`, stores required isolation properties and timestamps, and proves those behaviors with backend contract tests. It does not add HTTP routes, frontend integration, public query language surfaces, caller-supplied vectors, or LLM embedding orchestration.

## Assumptions

- The existing scope kernel remains the only source of namespace, root workspace, current workspace, lineage visibility, and sentinel normalization.
- The stable high-level `grafeo` crate is preferred over direct `grafeo-engine` coupling so internal callers stay insulated from unstable storage internals.
- The first production version uses embedded Grafeo only; Grafeo server mode and enterprise-only assumptions remain out of scope.
- LPG persistence can use Grafeo LPG capabilities; ontology persistence can use Grafeo RDF or equivalent Grafeo-native ontology storage capability, but the public Rust contracts remain typed and library-agnostic.
- If a required Grafeo capability is unavailable or weaker than the PRD requires, the adapter must return a typed unsupported-capability error instead of silently weakening isolation or persistence semantics.
- Adapter bootstrap must be idempotent so repeated service construction does not corrupt or duplicate schema objects.

## In Scope

- Add the Grafeo Rust dependency and the embedded feature set required for LPG plus ontology persistence.
- Add backend adapter modules under `tron-backend/src/adapters/outbound/persistence` for Grafeo graph and ontology storage.
- Add adapter bootstrap for one catalog, per-namespace schema selection, per-root-workspace named graph selection, and required graph indexes.
- Implement `GraphStore` CRUD, search, and traversal with resolved-scope isolation and required storage property writes.
- Implement `OntologyStore` CRUD and search with the same resolved-scope isolation and typed ontology results.
- Persist required graph isolation properties: `namespace`, `workspaceCode`, `appCode`, `userCode`, `createdAt`, and `updatedAt`.
- Add typed adapter error mapping so capability gaps and storage failures surface through graph-domain error taxonomies without leaking Grafeo types.
- Add runtime construction and app wiring for internal graph services without exposing any new HTTP route.
- Add backend contract and integration tests using temporary embedded Grafeo storage.

## Out Of Scope

- HTTP controllers, request DTOs, response DTOs, route registration, or frontend integration for graph or memory surfaces.
- LLM embedding generation, chunking, reranking, or vector orchestration beyond preserving the typed contract surface.
- Caller-supplied raw GQL, Cypher, SPARQL, Turtle, SHACL text, or embedding vectors.
- Cross-root named-graph joins, merged multi-root queries, Grafeo server deployment, or Grafeo enterprise-only features.
- Business features that consume graph storage, including knowledge graph UI, memory fragment APIs, retrieval ranking, or app/task/agent graph behaviors.

## Constraints

- One embedded Grafeo catalog is the only storage root for this feature.
- `ResolvedGraphScope.schema` maps to one Grafeo schema.
- `ResolvedGraphScope.named_graph` maps to one root-workspace Grafeo named graph.
- Adapter queries must never merge multiple root named graphs in one operation.
- Write operations must always persist `workspaceCode` as the current workspace code and `appCode` or `userCode` as concrete values or `__NONE__`.
- Read operations must honor `visible_workspace_codes` and must not widen visibility beyond the resolved lineage.
- Public domain and application signatures must not expose Grafeo-specific types or raw query strings.
- Unsupported Grafeo capabilities must raise typed unsupported behavior errors instead of silent fallbacks.
- Runtime wiring must keep the graph capability backend-internal and must not register `/api/v1/knowledge-graph` or `/api/v1/memory/fragments`.

## Feature Model

### Actors

- Internal graph caller: backend module that calls `GraphService` or `OntologyService`.
- Graph service and ontology service: existing typed application boundaries that validate and scope requests.
- Grafeo adapter: outbound persistence implementation for `GraphStore` and `OntologyStore`.
- Embedded Grafeo catalog: the only concrete graph storage backend in this slice.

### Core Objects

- `GrafeoGraphStore`: Grafeo-backed implementation of `GraphStore`.
- `GrafeoOntologyStore`: Grafeo-backed implementation of `OntologyStore`.
- `GrafeoAdapterConfig`: storage path and bootstrap configuration for the embedded catalog.
- `GrafeoCatalogManager`: idempotent catalog or schema bootstrap helper.
- `GrafeoScopeBinding`: adapter-level mapping from `ResolvedGraphScope` to Grafeo schema and named graph handles.
- `GraphRecordProjection`: adapter mapping for node and edge identifiers, labels, relationships, properties, text fields, and timestamps.
- `OntologyRecordProjection`: adapter mapping for class, property, domain, range, constraint, and shape resources.
- `GraphError` and `OntologyError`: typed error taxonomies with unsupported-capability and storage-failure mapping.

### Rules

- Scope resolution and contract validation still happen before the adapter receives a request.
- Adapter bootstrap is storage-local and idempotent.
- Required graph isolation properties are reserved and persisted on every stored graph record.
- Graph and ontology reads stay inside the resolved named graph and visible lineage.
- Adapter internals may use Grafeo-native query or RDF APIs, but those details never cross the `GraphStore` or `OntologyStore` boundary.
- Search and traversal mapping may execute only capabilities already represented by the typed graph contract; missing Grafeo support must surface as typed unsupported behavior.

### State Transitions

- app startup -> construct Grafeo adapter config -> initialize embedded catalog
- service call -> resolve scope -> bind Grafeo schema and named graph from scope
- valid write -> adapter persists LPG or ontology record with required properties and timestamps
- valid read or search -> adapter limits execution to the resolved named graph and visible lineage
- capability gap or storage failure -> adapter maps to typed graph or ontology error without leaking Grafeo internals

## Page And Module Surface

- page_surface: no frontend page and no HTTP route in this version
- internal_surface: existing graph domain and application modules remain typed and adapter-agnostic
- persistence_surface: Grafeo outbound adapter modules plus backend runtime construction
- likely_files_to_add: `tron-backend/src/adapters/outbound/persistence/grafeo.rs`; `tron-backend/src/adapters/outbound/persistence/grafeo/graph_store.rs`; `tron-backend/src/adapters/outbound/persistence/grafeo/ontology_store.rs`; `tron-backend/src/adapters/outbound/persistence/grafeo/bootstrap.rs`; `tron-backend/tests/graph_grafeo_adapter_contract.rs`
- likely_files_to_update: `tron-backend/Cargo.toml`; `tron-backend/src/adapters/outbound/persistence.rs`; `tron-backend/src/app.rs`; `tron-backend/src/config.rs`; `tron-backend/src/domain/graph/error.rs`
- test_surface: focused Grafeo adapter contract tests plus backend compile and runtime-wiring verification

## Repository Findings

- Current graph code stops at typed ports and fake stores; there is no production `GraphStore` or `OntologyStore` implementation.
- `tron-backend/src/app.rs` wires SQLite-backed services but does not construct graph services or graph adapters yet.
- Existing graph feature specs explicitly deferred Grafeo persistence to this missing feature workspace.
- The backend is synchronous today, so the first adapter may remain synchronous while still being `Send + Sync`.
- `tron-backend/Cargo.toml` currently has no Grafeo dependency and no graph storage config fields.

## Open Decisions

- None. Defaults adopted for execution: use the high-level `grafeo` crate, embedded mode only, a dedicated graph storage config path, typed unsupported-capability error mapping, and backend-only runtime wiring with no HTTP surface.
