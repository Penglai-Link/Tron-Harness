= PRD: Graph Storage Foundation

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron backend graph storage foundation.
- Problem: knowledge graph, ontology, memory, and future retrieval features need one internal graph persistence contract, but the backend currently has no graph bounded context or Grafeo adapter.
- Target users: backend developers, internal services, agent runtime consumers, and future knowledge or memory modules.
- Outcome: internal backend services can store, query, traverse, and evolve scoped LPG and RDF data through stable Rust abstractions backed by embedded Grafeo.

== Background
- Why now: workspace governance and model management are already present, and latest `master` adds `LlmRuntimeService` embedding calls that graph search can reuse without exposing vectors to callers.
- Current pain: graph-like frontend surfaces use mocks or reserved API strings, and backend modules would otherwise duplicate isolation, embedding, and storage rules.
- Business context: this is foundational infrastructure for the product-level `Knowledge, Memory, and Observability` area, but the first version is backend-only and has no HTTP or frontend contract.

== Goals
- Define one backend graph bounded context with domain records, application services, application ports, and an outbound Grafeo adapter.
- Support LPG CRUD, relationship traversal, structured filtering, full-text search, vector search, and hybrid search for business knowledge graphs.
- Support RDF ontology CRUD for classes, properties, domains, ranges, constraints, and SHACL shapes.
- Resolve graph visibility through namespace, workspace lineage, app, and user scope without crossing root workspace named graphs.
- Store one embedded Grafeo catalog, map `namespace` to Grafeo schema, and map root workspace code to Grafeo named graph.
- Normalize missing Grafeo isolation properties to `__NONE__` at the persistence boundary instead of writing null values.
- Derive embeddings inside graph services through `LlmRuntimeService::embedding_call` and `embedding_call_with_chunking` before vector writes or vector search.
- Keep Grafeo concrete APIs behind application ports so future storage changes do not affect internal callers.

== Scope
- In scope: backend Rust domain and application modules, GraphStore and OntologyStore ports, scope resolver, embedded Grafeo outbound adapter, LPG and RDF record contracts, internal service methods, embedding orchestration, error taxonomy, and backend tests.
- Out of scope: HTTP routes, frontend integration, public query language exposure, caller-supplied vectors as the normal API, Grafeo server mode, Grafeo enterprise-only assumptions, rerank quality tuning, graph visualization, and migration of current frontend mock pages.

== Feature Requirements
=== Module: Backend Graph Storage
==== Feature: Backend Graph Storage - Scope And Isolation Kernel
- name: Graph scope and visibility resolver.
- description: resolve namespace, current workspace, root workspace graph, visible workspace lineage, app boundary, and user boundary for all graph operations.
- user or role: backend graph services and internal modules.
- entry point: Rust `GraphScope`, `ScopeResolver`, and graph application service methods.
- trigger: any graph or ontology read or write command.
- preconditions: command supplies a nonblank namespace and current workspace code; workspace service can resolve an active workspace path.
- main flow: validate scope, load workspace detail, derive root workspace code from the first lineage item, derive visible workspace codes from the path, normalize missing app and user codes to `__NONE__` for storage, and pass one resolved scope to the store port.
- inputs and outputs: graph scope command in; resolved storage scope out with schema, named graph, visible workspace codes, and normalized storage dimensions.
- permissions or visibility: first version enforces data visibility through resolved graph scope only; no HTTP authorization boundary is introduced.
- related page headings: `Page: Backend API - Internal Graph Service`.
- related business rule headings: `Rule: Backend Graph Storage - Catalog Schema Graph Mapping`, `Rule: Backend Graph Storage - Workspace Lineage Visibility`, `Rule: Backend Graph Storage - Sentinel Normalization`.
- related exception headings: `Exception: Backend Graph Storage - Invalid Scope`, `Exception: Backend Graph Storage - Workspace Resolution Failure`.

==== Feature: Backend Graph Storage - LPG Store Contract
- name: Business knowledge graph store.
- description: provide stable internal operations for nodes, edges, labels, dynamic properties, structured filters, full-text clauses, traversal clauses, vector clauses, and hybrid search plans.
- user or role: future knowledge graph, memory, retrieval, app, task, and agent modules.
- entry point: `GraphService` and `GraphStore` application port.
- trigger: an internal caller creates, updates, deletes, fetches, searches, or traverses business graph records.
- preconditions: scope is resolved; node and edge identifiers are stable within the resolved graph; adapter can persist dynamic typed properties.
- main flow: validate command, apply scope, derive embeddings when vector indexing or vector search is requested, call the store port, and return graph-domain results without leaking Grafeo types.
- inputs and outputs: node, edge, search, or traversal commands in; graph records, traversal paths, or scored search results out.
- permissions or visibility: reads are limited to one root workspace named graph and the visible workspace lineage.
- related page headings: `Page: Backend API - Internal Graph Service`.
- related business rule headings: `Rule: Backend Graph Storage - Graph Record Boundary`, `Rule: Backend Graph Storage - Hybrid Query Composition`.
- related exception headings: `Exception: Backend Graph Storage - Duplicate Or Missing Graph Record`, `Exception: Backend Graph Storage - Storage Failure`.

==== Feature: Backend Graph Storage - Ontology Store Contract
- name: RDF ontology store.
- description: provide stable internal operations for ontology classes, properties, domains, ranges, constraints, and SHACL shapes across operations, development, test, and design ontology domains.
- user or role: backend ontology builders, future knowledge graph validation, and domain modeling modules.
- entry point: `OntologyService` or graph application service ontology methods plus `OntologyStore` application port.
- trigger: an internal caller creates, updates, deletes, fetches, or searches ontology resources.
- preconditions: scope is resolved; ontology resource identifiers are stable; requested ontology domain is valid or explicitly extended by the namespace.
- main flow: validate ontology command, apply scope, map ontology records through the store port, and return typed ontology records without exposing raw RDF query strings.
- inputs and outputs: ontology class, property, domain, range, constraint, and SHACL commands in; ontology resources and query results out.
- permissions or visibility: ontology data uses the same namespace, root workspace named graph, workspace lineage, app, and user boundaries as LPG data.
- related page headings: `Page: Backend API - Internal Graph Service`.
- related business rule headings: `Rule: Backend Graph Storage - Ontology Domain Boundary`, `Rule: Backend Graph Storage - Raw Query Shield`.
- related exception headings: `Exception: Backend Graph Storage - Ontology Resource Failure`, `Exception: Backend Graph Storage - Invalid Scope`.

==== Feature: Backend Graph Storage - Embedded Grafeo Adapter
- name: Grafeo persistence adapter.
- description: implement graph and ontology store ports on one embedded Grafeo catalog using namespace as schema and root workspace code as named graph.
- user or role: graph application services and backend maintainers.
- entry point: outbound adapter under `adapters/outbound/graph`.
- trigger: graph service calls a persistence or query port.
- preconditions: Grafeo embedded crate, feature flags, and APIs are verified during implementation; graph records have resolved storage scope.
- main flow: map domain commands to Grafeo LPG or RDF operations, create required indexes, store required isolation properties, preserve MVCC behavior where Grafeo provides it, and map adapter errors to graph errors.
- inputs and outputs: store port calls in; persisted records, search hits, traversal paths, ontology resources, or typed storage errors out.
- permissions or visibility: adapter never merges data across Grafeo named graphs in one query.
- related page headings: `Page: Backend API - Internal Graph Service`.
- related business rule headings: `Rule: Backend Graph Storage - Catalog Schema Graph Mapping`, `Rule: Backend Graph Storage - Required Grafeo Properties`, `Rule: Backend Graph Storage - Raw Query Shield`.
- related exception headings: `Exception: Backend Graph Storage - Grafeo Capability Gap`, `Exception: Backend Graph Storage - Storage Failure`.

==== Feature: Backend Graph Storage - Embedding And Hybrid Search Orchestration
- name: Internal embedding and hybrid search planner.
- description: derive write vectors and query vectors through the LLM runtime and combine structured filters, full-text clauses, vector clauses, and traversal constraints into one graph search request.
- user or role: internal search consumers, retrieval services, memory modules, and agent runtime tools.
- entry point: `GraphService` search/write methods and embedding helper.
- trigger: a write requests vector indexing, a long content write requires chunked vectors, or a search includes vector text.
- preconditions: graph command supplies text, not vectors; current workspace has an enabled embedding model or caller provides a valid model override.
- main flow: choose regular or chunked embedding call, convert runtime failures to graph embedding errors, pass vectors only to the store adapter, combine available search clauses, and return typed scored results.
- inputs and outputs: text-bearing graph commands and search queries in; vector-enriched store requests and graph search results out.
- permissions or visibility: embeddings are resolved for the current workspace and do not bypass graph scope visibility.
- related page headings: `Page: Backend API - Internal Graph Service`.
- related business rule headings: `Rule: Backend Graph Storage - Internal Embedding Ownership`, `Rule: Backend Graph Storage - Hybrid Query Composition`.
- related exception headings: `Exception: Backend Graph Storage - Embedding Unavailable`, `Exception: Backend Graph Storage - Embedding Provider Failure`.

== Page Prototypes And Interactions
=== Page: Backend API - Internal Graph Service
- entry: no HTTP route or frontend page in this version.
- layout: Rust modules under `domain/graph`, `application/graph`, and `adapters/outbound/graph`, following the existing no-`mod.rs` backend style.
- data display: not applicable.
- actions: create, update, delete, fetch, search, traverse, and ontology mutation/query through internal service methods.
- states: scope resolved, embedding requested, embedding completed, store request built, Grafeo operation completed, search results returned, typed error returned.

==== Interaction: Backend API - Store Graph Record
- trigger control: internal caller invokes a graph service write method.
- preconditions: nonblank namespace and workspace code, valid node or edge command, and optional app/user/model metadata.
- button or control behavior: not applicable.
- form rules: dynamic record properties are accepted through typed backend values; missing app or user scope remains optional at the command boundary.
- confirmation behavior: not applicable.
- success feedback: service returns the stored graph record or mutation result.
- failure feedback: service returns a typed graph error.
- navigation or refresh behavior: not applicable.

==== Interaction: Backend API - Search Graph
- trigger control: internal caller invokes a graph service search method.
- preconditions: resolved scope; at least one structured, full-text, vector text, or traversal clause; enabled embedding model when vector text is present.
- button or control behavior: not applicable.
- form rules: callers supply vector text, never raw embedding vectors as the normal API contract.
- confirmation behavior: not applicable.
- success feedback: service returns scoped graph search results with scores when available.
- failure feedback: vector requests fail with embedding errors when no embedding model is available or the provider fails; the service does not silently run a weaker query.
- navigation or refresh behavior: not applicable.

==== Interaction: Backend API - Manage Ontology Resource
- trigger control: internal caller invokes an ontology store method through the graph application service.
- preconditions: resolved scope and valid ontology resource command.
- button or control behavior: not applicable.
- form rules: ontology domain must be one of operations, development, test, design, or a namespace-defined extension.
- confirmation behavior: not applicable.
- success feedback: service returns the stored ontology resource or query result.
- failure feedback: service returns a typed ontology or storage error.
- navigation or refresh behavior: not applicable.

== Business Rules
=== Rule: Backend Graph Storage - Catalog Schema Graph Mapping
- condition: graph data is persisted or queried.
- decision or calculation: use one embedded Grafeo catalog, map `namespace` to schema, and map root workspace code to named graph.
- state transition: a write creates or updates data only inside the resolved schema and named graph.
- permission or data boundary: a single read never crosses root workspace named graphs.
- conflict or priority rule: namespace/schema and root workspace/named graph mapping wins over caller-provided physical storage hints.
- example: workspace `WS-TEAM` under root `WS-ROOT` writes to schema `ops` and graph `WS-ROOT`.

=== Rule: Backend Graph Storage - Workspace Lineage Visibility
- condition: a read occurs from a child workspace.
- decision or calculation: visible workspace codes are the workspace path from root to current workspace.
- state transition: writes store the current workspace code; reads include ancestors and current workspace only.
- permission or data boundary: sibling and child workspace data outside the current path is not visible.
- conflict or priority rule: workspace path returned by `WorkspaceService::get_workspace` is authoritative.
- example: root data can be seen by its child; sibling workspace data cannot be seen from the child.

=== Rule: Backend Graph Storage - Sentinel Normalization
- condition: app or user scope is absent at the command boundary.
- decision or calculation: persist `appCode` or `userCode` as `__NONE__`, never null, in Grafeo properties.
- state transition: domain command DTOs keep optional values, while adapter payloads store concrete strings.
- permission or data boundary: sentinel values participate in indexed filtering like normal values.
- conflict or priority rule: no graph domain record stores an `inheritable` flag or equivalent persistence switch.
- example: namespace-level data with no app or user writes `appCode="__NONE__"` and `userCode="__NONE__"`.

=== Rule: Backend Graph Storage - Required Grafeo Properties
- condition: a node or edge is written.
- decision or calculation: every persisted node and edge includes `namespace`, `workspaceCode`, `appCode`, `userCode`, `createdAt`, and `updatedAt`.
- state transition: adapter creates or maintains indexes for `workspaceCode`, `appCode`, and `userCode`.
- permission or data boundary: required properties support diagnostics, export, and scope filtering.
- conflict or priority rule: business dynamic properties cannot override required isolation property names.
- example: a node property map that attempts to set `workspaceCode` as business data is rejected or namespaced before persistence.

=== Rule: Backend Graph Storage - Graph Record Boundary
- condition: an internal module writes LPG data.
- decision or calculation: records use stable node or edge codes, labels, typed dynamic properties, optional full-text text, and optional embedding text.
- state transition: create, update, and delete operations maintain graph consistency for source and target node references.
- permission or data boundary: source and target nodes must exist in the same resolved schema and named graph.
- conflict or priority rule: graph service validation runs before Grafeo adapter calls.
- example: an edge from a task node to an app node can be created only when both nodes resolve in the same root workspace graph.

=== Rule: Backend Graph Storage - Ontology Domain Boundary
- condition: ontology data is written or queried.
- decision or calculation: ontology resources are typed as class, property, domain, range, constraint, or SHACL shape.
- state transition: valid ontology domains include operations, development, test, design, and namespace-defined extensions.
- permission or data boundary: ontology data uses the same graph scope isolation as LPG data.
- conflict or priority rule: raw RDF statements are not accepted from callers in the first version.
- example: a development-domain class can define properties and ranges without sharing data with a separate root workspace graph.

=== Rule: Backend Graph Storage - Internal Embedding Ownership
- condition: vector indexing or vector search is requested.
- decision or calculation: graph service derives embeddings by calling `LlmRuntimeService` with the current workspace code and optional model override.
- state transition: embedding vectors are passed to the store adapter but are not part of normal caller input.
- permission or data boundary: embedding model resolution follows existing workspace-aware model management.
- conflict or priority rule: if no embedding model is available, return a graph embedding error instead of running a weaker non-vector query.
- example: a search query with vector text calls `embedding_call` before the Grafeo adapter receives the search request.

=== Rule: Backend Graph Storage - Hybrid Query Composition
- condition: search includes multiple retrieval dimensions.
- decision or calculation: combine structured filters, full-text search, vector search, traversal seeds, labels, limits, and score options in one typed search plan.
- state transition: absent dimensions are skipped; requested dimensions must either execute or fail explicitly.
- permission or data boundary: all search dimensions are evaluated within the resolved schema, named graph, and visible workspace codes.
- conflict or priority rule: future rerank may refine results through `LlmRuntimeService::rerank_call`, but rerank is not required for the first storage foundation.
- example: a memory query can filter by app, traverse from a task node, and rank by vector similarity in the same scoped query.

=== Rule: Backend Graph Storage - Raw Query Shield
- condition: an internal caller requests graph or ontology behavior.
- decision or calculation: callers use typed commands and queries, not raw GQL, Cypher, SPARQL, or Grafeo query strings.
- state transition: only the outbound adapter maps typed requests to Grafeo-specific calls.
- permission or data boundary: typed requests prevent callers from bypassing scope filters.
- conflict or priority rule: adapter-specific query features require a typed application-level representation before use.
- example: ontology search accepts class/property filters, not a caller-provided SPARQL string.

== Exceptions And Boundaries
=== Exception: Backend Graph Storage - Invalid Scope
- scenario: namespace or workspace code is blank, malformed, archived, or cannot produce a valid workspace path.
- expected product behavior: service returns a typed graph scope error before calling Grafeo.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: no graph or ontology write occurs.
- retry, recovery, or compensation behavior: caller fixes scope inputs or workspace state and retries.
- logging/audit expectation: log scope error with namespace and workspace code when available, without dynamic record payloads.

=== Exception: Backend Graph Storage - Workspace Resolution Failure
- scenario: `WorkspaceService` cannot load active workspace detail or lineage.
- expected product behavior: service maps the workspace failure to a graph scope or storage preparation error.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: no store port call occurs.
- retry, recovery, or compensation behavior: retry only after workspace service recovers or workspace access is corrected.
- logging/audit expectation: record the workspace code and failure category.

=== Exception: Backend Graph Storage - Duplicate Or Missing Graph Record
- scenario: a create command uses an existing node or edge code, an update/delete references a missing record, or an edge references missing endpoints.
- expected product behavior: service returns a typed duplicate, missing node, or missing edge error.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: failed mutations do not partially update related records.
- retry, recovery, or compensation behavior: caller may fetch current records, choose new codes, or create missing endpoints.
- logging/audit expectation: include record type and code, not full dynamic property payload.

=== Exception: Backend Graph Storage - Ontology Resource Failure
- scenario: ontology class, property, domain, range, constraint, or SHACL shape is missing, duplicate, invalid, or inconsistent.
- expected product behavior: service returns a typed ontology error.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: invalid ontology mutations do not corrupt related RDF resources.
- retry, recovery, or compensation behavior: caller corrects the ontology command and retries.
- logging/audit expectation: include ontology resource type and code.

=== Exception: Backend Graph Storage - Embedding Unavailable
- scenario: vector write or vector search is requested but no enabled embedding model resolves for the workspace and optional model override.
- expected product behavior: service returns a graph embedding unavailable error and does not degrade to structured or full-text search.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: vector-indexed writes do not persist without their required vector payload.
- retry, recovery, or compensation behavior: configure an embedding model or remove the vector requirement.
- logging/audit expectation: include workspace code and optional model code, not the input text.

=== Exception: Backend Graph Storage - Embedding Provider Failure
- scenario: the LLM runtime embedding provider times out, rejects a request, or returns an invalid vector.
- expected product behavior: service maps the runtime error to a graph embedding provider failure.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: vector-dependent writes and searches fail atomically at the graph service boundary.
- retry, recovery, or compensation behavior: caller may retry after provider recovery; no automatic fallback is assumed.
- logging/audit expectation: preserve provider failure category and model code without raw text.

=== Exception: Backend Graph Storage - Grafeo Capability Gap
- scenario: embedded Grafeo crate API, feature flags, vector index support, RDF behavior, or hybrid query behavior differs from design assumptions.
- expected product behavior: adapter narrows implementation to verified embedded capabilities and exposes typed unsupported-capability errors where needed.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: unsupported features do not create partial or misleading persisted state.
- retry, recovery, or compensation behavior: implementation may add a typed adapter fallback only if it preserves scope and query semantics.
- logging/audit expectation: record capability name, crate version, and operation type.

=== Exception: Backend Graph Storage - Storage Failure
- scenario: Grafeo operation, index setup, serialization, MVCC handling, or adapter mapping fails.
- expected product behavior: service returns a typed storage failure.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: mutations follow Grafeo atomicity/MVCC guarantees where available; otherwise adapter reports the uncertain state.
- retry, recovery, or compensation behavior: caller may retry only when failure category is retryable.
- logging/audit expectation: log adapter operation and resolved schema/named graph, without sensitive dynamic properties.

== Shared Requirements
- Workspace lineage resolver: shared because graph, ontology, memory, and future retrieval modules all need root workspace graph selection and visible ancestor scope; consumers: Scope And Isolation Kernel, LPG Store Contract, Ontology Store Contract, Embedded Grafeo Adapter, Embedding And Hybrid Search Orchestration; decision: build first by reusing `WorkspaceService::get_workspace`.
- LLM embedding runtime: shared because vector write and vector search must derive vectors through one workspace-aware runtime; consumers: LPG Store Contract and Embedding And Hybrid Search Orchestration; decision: already available on latest `master`, integrate before vector ACs.
- Typed graph query contract: shared because callers must not bypass scope rules with raw graph query strings; consumers: LPG Store Contract, Ontology Store Contract, and Embedded Grafeo Adapter; decision: build before exposing hybrid search.
- Grafeo adapter capability verification: shared because embedded crate details decide index, vector, RDF, MVCC, and query mapping shape; consumers: Embedded Grafeo Adapter, LPG Store Contract, Ontology Store Contract, and Embedding And Hybrid Search Orchestration; decision: verify during the first adapter slice.
- HTTP and frontend API contract: shared future requirement for current graph, memory, and knowledge-base mock surfaces; consumers: future graph UI integration and future memory retrieval; decision: defer, because first version is backend-internal only.

== Feature Inventory
1. Graph Scope And Isolation Kernel: module Backend Graph Storage; source heading `Feature: Backend Graph Storage - Scope And Isolation Kernel`; page `Page: Backend API - Internal Graph Service`; rules `Rule: Backend Graph Storage - Catalog Schema Graph Mapping`, `Rule: Backend Graph Storage - Workspace Lineage Visibility`, `Rule: Backend Graph Storage - Sentinel Normalization`, `Rule: Backend Graph Storage - Required Grafeo Properties`; exceptions `Exception: Backend Graph Storage - Invalid Scope`, `Exception: Backend Graph Storage - Workspace Resolution Failure`; goal create the reusable backend scope, visibility, and sentinel foundation; in scope Rust domain/application scope types, resolver tests, and storage-scope outputs; out of scope Grafeo persistence and HTTP; depends on existing workspace governance.
2. Graph Store Port And LPG Domain Contract: module Backend Graph Storage; source heading `Feature: Backend Graph Storage - LPG Store Contract`; page `Page: Backend API - Internal Graph Service`; rules `Rule: Backend Graph Storage - Graph Record Boundary`, `Rule: Backend Graph Storage - Raw Query Shield`; exceptions `Exception: Backend Graph Storage - Duplicate Or Missing Graph Record`; goal define typed LPG records, CRUD/search/traverse commands, and GraphStore port; in scope internal Rust contracts and service validation; out of scope concrete Grafeo execution until adapter slice; depends on Feature 1.
3. Ontology Store Port And RDF Domain Contract: module Backend Graph Storage; source heading `Feature: Backend Graph Storage - Ontology Store Contract`; page `Page: Backend API - Internal Graph Service`; rules `Rule: Backend Graph Storage - Ontology Domain Boundary`, `Rule: Backend Graph Storage - Raw Query Shield`; exceptions `Exception: Backend Graph Storage - Ontology Resource Failure`; goal define typed ontology records and OntologyStore port; in scope class, property, domain, range, constraint, and SHACL contracts; out of scope public SPARQL and UI; depends on Feature 1.
4. Embedded Grafeo Adapter Foundation: module Backend Graph Storage; source heading `Feature: Backend Graph Storage - Embedded Grafeo Adapter`; page `Page: Backend API - Internal Graph Service`; rules `Rule: Backend Graph Storage - Catalog Schema Graph Mapping`, `Rule: Backend Graph Storage - Required Grafeo Properties`, `Rule: Backend Graph Storage - Raw Query Shield`; exceptions `Exception: Backend Graph Storage - Grafeo Capability Gap`, `Exception: Backend Graph Storage - Storage Failure`; goal implement GraphStore and OntologyStore on embedded Grafeo with one catalog, namespace schema, root workspace named graph, required properties, and indexes; in scope verified embedded APIs, LPG/RDF persistence, adapter mapping, and contract tests; out of scope Grafeo server and enterprise-only features; depends on Feature 2 and Feature 3.
5. Embedding And Hybrid Search Orchestration: module Backend Graph Storage; source heading `Feature: Backend Graph Storage - Embedding And Hybrid Search Orchestration`; page `Page: Backend API - Internal Graph Service`; rules `Rule: Backend Graph Storage - Internal Embedding Ownership`, `Rule: Backend Graph Storage - Hybrid Query Composition`; exceptions `Exception: Backend Graph Storage - Embedding Unavailable`, `Exception: Backend Graph Storage - Embedding Provider Failure`, `Exception: Backend Graph Storage - Storage Failure`; goal derive vectors through LLM runtime and execute scoped hybrid search; in scope write vectors, query vectors, chunked embedding, structured/full-text/vector/traversal composition, and no-silent-degrade errors; out of scope rerank quality tuning; depends on Feature 1, Feature 2, Feature 4, and existing LLM runtime.

== Dependency Map
#mermaid(
  "flowchart LR
    EF1[\"Existing Foundation: Workspace Governance\"] --> F1[\"Feature 1: Graph Scope And Isolation Kernel\"]
    EF2[\"Existing Foundation: LLM Runtime Embeddings\"] --> F5[\"Feature 5: Embedding And Hybrid Search Orchestration\"]
    F1 --> F2[\"Feature 2: Graph Store Port And LPG Domain Contract\"]
    F1 --> F3[\"Feature 3: Ontology Store Port And RDF Domain Contract\"]
    F2 --> F4[\"Feature 4: Embedded Grafeo Adapter Foundation\"]
    F3 --> F4
    F1 --> F5
    F2 --> F5
    F4 --> F5
    F5 --> FF1[\"Future: HTTP and Frontend Knowledge Surfaces\"]"
)

== Recommended Implementation Order
1. Existing foundation verification - confirm workspace lineage, model management embedding resolution, LLM runtime embedding calls, and Grafeo embedded crate capabilities against latest `master`.
2. Graph Scope And Isolation Kernel - this is the first bounded feature because every LPG, RDF, adapter, and search behavior depends on the same scope and sentinel rules.
3. Graph Store Port And LPG Domain Contract - define the stable business graph API before binding callers to Grafeo.
4. Ontology Store Port And RDF Domain Contract - define RDF ontology behavior with the same isolation model before adapter expansion.
5. Embedded Grafeo Adapter Foundation - implement verified embedded Grafeo storage behind the ports once typed contracts are stable.
6. Embedding And Hybrid Search Orchestration - add vector and hybrid behavior after scope, LPG contract, and adapter capability are proven.
7. Future HTTP and frontend integration - expose product pages only after backend contracts have tests and internal consumers can use them.

== Acceptance
=== Acceptance: Backend Graph Storage - Scope And Isolation Kernel
- Backend tests prove namespace maps to schema, root workspace maps to named graph, current workspace writes store exact workspace code, and reads use visible workspace lineage only.
- Backend tests prove missing app and user codes are normalized to `__NONE__` only at the persistence boundary.
- Domain and adapter payloads contain no `inheritable` field or equivalent persistence flag.

=== Acceptance: Backend Graph Storage - LPG Store Contract
- Backend tests prove typed node and edge CRUD command validation, duplicate and missing record errors, and source/target graph consistency.
- Backend tests prove structured filter, full-text, vector text, traversal, and hybrid query contracts are represented without raw Grafeo query strings.
- Internal callers do not supply embedding vectors as the normal write or search contract.

=== Acceptance: Backend Graph Storage - Ontology Store Contract
- Backend tests prove ontology class, property, domain, range, constraint, and SHACL commands validate and return typed ontology results.
- Ontology data uses the same namespace, root workspace graph, workspace lineage, app, and user scope rules as LPG data.
- Raw RDF or SPARQL strings are not accepted from callers in the first version.

=== Acceptance: Backend Graph Storage - Embedded Grafeo Adapter
- Adapter contract tests prove one embedded Grafeo catalog, namespace schema mapping, root workspace named graph mapping, required node/edge properties, and required scope indexes.
- Adapter tests prove LPG and RDF operations stay inside one resolved named graph and never merge root workspace graphs in one query.
- If an embedded Grafeo capability is unavailable, adapter behavior returns a typed unsupported-capability error instead of weakening scope or persistence semantics.

=== Acceptance: Backend Graph Storage - Embedding And Hybrid Search Orchestration
- Service tests prove vector-indexed writes and vector searches call `LlmRuntimeService::embedding_call`, and chunked writes call `embedding_call_with_chunking`.
- Service tests prove vector search with no available embedding model fails with a graph embedding error and does not silently degrade.
- Search tests prove structured filters, full-text search, vector search, and graph traversal can be combined under the resolved scope.

== Risks And Open Questions
- Grafeo embedded crate name, feature flags, LPG/RDF/vector APIs, and MVCC behavior must be verified during implementation; prior crate lookup had network SSL failures.
- Grafeo enterprise/server-only features must not be assumed for the first embedded version.
- Chunk vector storage shape may require adapter-specific records once Grafeo vector APIs are verified.
- Dynamic property typing must avoid collisions with required isolation property names.
- Future frontend graph, memory, and knowledge-base routes currently imply HTTP endpoints, but those endpoints stay out of scope for this backend-only PRD.
