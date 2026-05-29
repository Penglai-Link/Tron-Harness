# Feature Spec: Graph Scope And Isolation Kernel

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: graph-scope-and-isolation-kernel
- feature_name: Graph Scope And Isolation Kernel
- status: approved_with_defaults
- owner: clouds3n
- source_request: add a pure backend graph foundation using one embedded Grafeo catalog, namespace schema mapping, workspace-root named graphs, sentinel storage normalization, and internal LLM embedding ownership in later graph slices
- main_agent: lead agent on `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/base`
- base_branch: `feature/clouds3n/graph-storage-foundation/base`
- base_commit: `a73a1b51eae48033826bbc0cc0e9ea7465cc3a83`
- delivery_type: new_feature
- dependency_features: existing workspace governance; later graph-store-lpg-domain-contract; later ontology-store-rdf-domain-contract; later embedded-grafeo-adapter-foundation; later embedding-hybrid-search-orchestration
- dependency_contracts: `WorkspaceService::get_workspace`, `WorkspaceDetail::path`, `WorkspaceStatus::Active`, Rust facade modules `domain.rs` and `application.rs`, backend no-`mod.rs` architecture test, PRD graph storage feature inventory

## Requirement

Tron needs one backend graph scope kernel that turns an internal graph or ontology command into a resolved storage scope before any store port, Grafeo adapter, ontology contract, or hybrid search planner runs. The kernel must validate namespace and workspace inputs, derive namespace-to-schema and root-workspace-to-named-graph mapping, calculate visible workspace lineage, normalize absent app and user dimensions to `__NONE__` only for storage payloads, and keep HTTP and concrete Grafeo persistence out of the first slice.

## Product Source Slices

- selected_feature: `Feature: Backend Graph Storage - Scope And Isolation Kernel`
- selected_page: `Page: Backend API - Internal Graph Service`
- selected_rules: `Rule: Backend Graph Storage - Catalog Schema Graph Mapping`; `Rule: Backend Graph Storage - Workspace Lineage Visibility`; `Rule: Backend Graph Storage - Sentinel Normalization`; `Rule: Backend Graph Storage - Required Grafeo Properties`
- selected_exceptions: `Exception: Backend Graph Storage - Invalid Scope`; `Exception: Backend Graph Storage - Workspace Resolution Failure`
- selected_acceptance_source: `Acceptance: Backend Graph Storage - Scope And Isolation Kernel`
- selected_workspace: new `docs/features/graph-scope-and-isolation-kernel`
- feature_inventory_summary: scope kernel first; LPG and ontology contracts depend on it; embedded Grafeo adapter depends on LPG plus ontology contracts; embedding and hybrid search depends on scope, LPG, adapter, and existing LLM runtime
- shared_requirement_summary: workspace lineage resolver is built first here; typed graph query contract, Grafeo capability verification, and LLM embedding runtime integration stay in later feature workspaces
- dependency_map_summary: existing workspace governance feeds this feature; this feature feeds LPG, ontology, Grafeo adapter, and hybrid search; HTTP and frontend graph surfaces remain future work
- implementation_order_summary: verify existing foundations, build scope kernel, then build LPG contract, ontology contract, Grafeo adapter, embedding and hybrid search, and only then future HTTP or frontend integration

## Scope Boundary

This feature owns domain and application types for graph scope resolution plus tests that prove the derived storage scope is correct. It does not create graph records, ontology records, Grafeo storage, vector embeddings, hybrid search, HTTP endpoints, frontend mocks, or public API contracts. Later features consume this kernel when they define `GraphStore`, `OntologyStore`, Grafeo adapters, and embedding-aware search orchestration.

## Assumptions

- `WorkspaceService::get_workspace` is the authoritative source for workspace lineage and returns the path from root workspace to current workspace.
- `WorkspaceService::get_workspace` can return archived workspace detail, so this kernel must reject a non-active current workspace before producing a graph storage scope.
- Namespace is a storage namespace, not an HTTP path segment; the first implementation can require a trimmed, lowercase, storage-safe identifier.
- App and user dimensions are optional isolation filters, not existence checks in this slice.
- Blank optional app or user values are equivalent to absent command fields and normalize to the storage sentinel.
- The storage sentinel value `__NONE__` is reserved and cannot be used as an explicit app or user code.
- The current workspace code is stored exactly as the write workspace. Visible read scope is the lineage path, not descendants or siblings.
- The first slice exposes a resolved storage-scope payload shape for future adapters. It does not assert concrete Grafeo APIs.

## In Scope

- `domain/graph.rs` facade and small graph domain modules for scope input, resolved scope, storage dimensions, required property names, and graph scope errors.
- `application/graph.rs` facade and small graph application modules for `ScopeResolver` behavior.
- A resolver that validates nonblank namespace and workspace code before calling workspace services.
- Workspace detail resolution through existing `WorkspaceService::get_workspace`.
- Root workspace code derivation from the first lineage item.
- Visible workspace code derivation from the full lineage path from root to current workspace.
- Storage schema derivation from namespace and named graph derivation from root workspace code.
- Storage dimension normalization from optional app and user command values into concrete `appCode` and `userCode` storage values.
- Required graph isolation property names for later node and edge payloads: `namespace`, `workspaceCode`, `appCode`, `userCode`, `createdAt`, and `updatedAt`.
- Tests for valid scope resolution, child workspace lineage visibility, invalid inputs, archived current workspace rejection, workspace resolution failure mapping, sentinel normalization, and reserved property protection.

## Out Of Scope

- HTTP routes, controllers, request and response DTOs, frontend pages, or frontend API mock changes.
- Grafeo dependency declaration, catalog creation, schema creation, named graph creation, indexes, MVCC, LPG persistence, or RDF persistence.
- `GraphStore`, `OntologyStore`, node, edge, label, ontology class, property, SHACL, traversal, or search contracts.
- LLM runtime embedding calls, chunking, vector storage, reranking, or hybrid query planning.
- Authorization, role checking, app existence checking, user existence checking, or cross-module permission policy.
- Cross-root graph merging or querying more than one root workspace named graph in one operation.
- Persisting an `inheritable` field or any equivalent switch.

## Constraints

- First version remains pure backend and internal Rust only.
- One Grafeo catalog is assumed by later adapter work; this slice only emits schema and named graph identifiers.
- Namespace maps to schema; root workspace code maps to named graph.
- A read scope can include only the visible workspace lineage returned by the workspace path.
- A write scope stores only the current workspace code.
- `appCode` and `userCode` must be concrete strings in storage payloads and must never be null.
- The sentinel `__NONE__` is applied at the storage boundary, not forced into caller-facing optional command fields.
- Domain types must not import application, adapter, HTTP, SQLite, request client, or event-bus boundaries.
- Backend module layout must avoid `mod.rs` and keep facade files small.

## Feature Model

### Actors

- Internal graph caller: a backend module that will later read or write graph or ontology data.
- Scope resolver: application service boundary that resolves command scope into storage scope.
- Workspace service: existing application service that provides workspace detail and lineage.
- Future store adapter: later Grafeo-backed component that consumes the resolved storage scope.

### Core Objects

- `GraphScopeCommand`: caller-supplied namespace, current workspace code, optional app code, and optional user code.
- `GraphScope`: validated domain command scope with optional app and user dimensions still optional.
- `ResolvedGraphScope`: resolved read/write scope with schema, named graph, current workspace, root workspace, visible workspace codes, and storage dimensions.
- `GraphStorageDimensions`: concrete storage values for `namespace`, `workspaceCode`, `appCode`, and `userCode`.
- `GraphStorageProperty`: reserved property name contract for future node and edge payloads.
- `GraphScopeError`: typed error for invalid scope, archived workspace, empty lineage, workspace resolution failure, and reserved sentinel misuse.
- `ScopeResolver`: application component that uses `WorkspaceService` and graph domain rules to produce `ResolvedGraphScope`.

### Rules

- Namespace is trimmed and validated before any workspace lookup.
- Workspace code is trimmed and validated before any workspace lookup.
- Current workspace detail must be active.
- Workspace path must be nonempty.
- `root_workspace_code` is the first path item.
- `visible_workspace_codes` is every path item in returned order, preserving root-to-current order.
- `schema` equals the validated namespace.
- `named_graph` equals the root workspace code.
- `workspaceCode` equals the current workspace code for writes.
- Optional app and user values are trimmed; empty values become absent; absent values become `__NONE__` in storage dimensions.
- Explicit `__NONE__` in app or user command values is rejected so callers cannot confuse real IDs with absence.
- Required storage property names are reserved and cannot be overwritten by future dynamic properties.

### State Transitions

- caller submits raw command scope -> domain validation returns `GraphScope`
- resolver loads workspace detail -> rejects inactive or malformed workspace detail
- workspace path resolves -> root and visible workspace codes are derived
- optional dimensions normalize -> storage dimensions contain no null-equivalent values
- storage scope returns -> future store port receives one scope object and no physical storage hints from callers

## Page And Module Surface

- page_surface: no frontend page and no HTTP route in this version
- internal_surface: Rust modules under `tron-backend/src/domain/graph` and `tron-backend/src/application/graph`
- command_surface: internal graph and ontology services will call the resolver before store operations in later slices
- persistence_surface: none in this feature; future Grafeo adapter consumes `schema`, `named_graph`, visible workspace codes, and storage dimensions
- likely_files_to_add: `tron-backend/src/domain/graph.rs`; `tron-backend/src/domain/graph/scope.rs`; `tron-backend/src/domain/graph/error.rs`; `tron-backend/src/application/graph.rs`; `tron-backend/src/application/graph/scope_resolver.rs`
- likely_files_to_update: `tron-backend/src/domain.rs`; `tron-backend/src/application.rs`
- test_surface: focused Rust tests near graph domain/application modules plus existing backend architecture tests

## Repository Findings

- `tron-backend/src/domain.rs` and `tron-backend/src/application.rs` are small facade modules and should export `graph` without becoming implementation buckets.
- Existing Rust architecture tests forbid backend `mod.rs`, oversized facade modules, and domain imports from infrastructure or application boundaries.
- `WorkspaceService::get_workspace` already returns `WorkspaceDetail` with `path`, which is enough to derive root workspace code and visible lineage.
- `WorkspaceDetail` includes current workspace `status`, but `WorkspaceLineageItem` does not include ancestor status; this feature can reject the current archived workspace and trust the workspace path for lineage.
- `LlmRuntimeService` is already wired in `AppState`, but this selected feature does not call it because embedding belongs to later vector and hybrid search slices.
- `tron-backend/Cargo.toml` currently has no Grafeo dependency; this feature should not add it.
- Frontend code already contains knowledge graph and memory endpoint strings, but this backend foundation must not add HTTP APIs in the first version.

## Open Decisions

- None. Defaults adopted for execution: namespace uses lowercase ASCII letters, digits, `_`, and `-`; the first implementation rejects archived current workspace and defers ancestor status enforcement because `WorkspaceDetail::path` has no status field.
