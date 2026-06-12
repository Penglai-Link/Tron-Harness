# Feature Spec: RAG 管理配置 API 接入

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

本稿复用已经和用户对齐的阶段性方案：先把现有 RAG 模块的 Corpus/Profile 管理页面从浏览器本地 mock 数据切换到后端真实管理接口，同时保持 ingest/search/debug/retry 运行态契约不被本次改造打断；随后补齐 SQLite 独立 `vectorTable` 的 runtime execution support，让管理态分离配置和运行态向量链路形成闭环。当前前端已经完成一版 mock 配置流，本 feature 的目标是补齐后端管理接口、持久化兼容层、前端 API 接入和验证链路。

## Feature Identity And Context

- feature_slug: rag-management-api-integration
- feature_name: RAG 管理配置 API 接入
- status: approved_with_defaults
- owner: xinlangtao
- source_request: 将 RAG 模块现有语料库和 profile 配置页面从前端 mock 数据切换为后端真实接口，按 codex-superpower 进行 AC 拆分、worktree 隔离、多 agent 并行和 TDD 交付。
- latest_evolution_request: 继续基于已交付的 phase-1 management API 和 SQLite 独立 `vectorTable` runtime，升级为 role-aware storage：`source`、`textUnit`、`fulltext`、`vector` 可以按业务需要绑定到不同 storage target，runtime 写入和检索按 segment/backend 分发。
- main_agent: lead agent on `feature/xinlangtao/rag-frontend-release-0.1.0/base`
- dependency_features: rag-corpus-storage-profile-foundation, rag-corpus-storage-structure-form, rag-diagnostics-retry-trace-events, shared-ui-feedback
- dependency_contracts: `tron-backend/src/response.rs`, `tron-backend/src/adapters/inbound/http/rag/*`, `tron-backend/src/application/rag/*`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-frontend/src/shared/api/httpClient.ts`, `tron-frontend/src/shared/api/serviceConfig.ts`

## Requirement

- user_story: 作为 RAG 管理员，我需要在管理页面中查看、创建、编辑 Corpus 与 Ingest/Retrieval Profile，并让这些配置真正落到后端而不是停留在浏览器本地 mock，这样管理页面、后端存储和后续运行时之间才能形成闭环。
- intent: 先交付 RAG 管理态接口和前端接入能力，保持当前前端设计方向不变，并通过兼容层把新的 target-first storage 设计安全映射到现有 runtime 仍然依赖的 legacy storage binding；随后将 runtime 从 whole-corpus backend dispatch 升级为 role-aware dispatch，让 `fulltext` 和 `vector` 检索载体可以独立于 `textUnit` 主数据存储。
- outcome: 后端新增 RAG corpora/profiles/catalog/verify 管理接口，前端 RAG 列表与表单页改为 API 数据源，用户可以通过真实接口完成配置流；同时现有 ingest/search/debug/retry 在 SQLite split corpus 下能够真正读取独立 `vectorTable`，并在本次演进中支持独立 `fulltextTable` 与 SQLite/Graph 检索载体组合。

## Product Source Slices

- selected_feature: `Feature: RAG Management API Integration`
- selected_page: `Page: /rag?tab=corpora`, `Page: /rag/corpora/new`, `Page: /rag/corpora/:corpusId/edit`, `Page: /rag?tab=profiles`, `Page: /rag/profiles/ingest/new`, `Page: /rag/profiles/retrieval/new`
- selected_interactions: `Interaction: Load RAG corpora list from API`, `Interaction: Create corpus with target-first storage`, `Interaction: Edit corpus and preserve metadata groups`, `Interaction: Create ingest profile`, `Interaction: Create retrieval profile with multiple corpora`
- selected_rules: `Rule: target-first storage topology persists separately from runtime legacy binding`, `Rule: graph capability requires bound graph target when enabled`, `Rule: metadataGroups must not be dropped by management edits`, `Rule: profile strategy options are served by catalog rather than hardcoded in the page`
- selected_exceptions: `Exception: unsupported mixed storage topology`, `Exception: graph enabled without graph-backed bound segment`, `Exception: unknown filter field or unsupported operator`, `Exception: unsupported strategy id`, `Exception: missing runtime-compatible projection`
- selected_acceptance_source: `Acceptance: RAG 管理配置 API 接入`
- selected_workspace: `docs/features/rag-management-api-integration`
- feature_inventory_summary: this slice covers management contracts, page integration, and the follow-up SQLite distinct-`vectorTable` runtime execution closure needed by the approved corpus storage UX
- shared_requirement_summary: backend remains the source of truth for response envelope, validation, catalog options, compatibility rules, and SQLite vector execution behavior; frontend preserves the approved storage/profile UX while sourcing data from APIs
- dependency_map_summary: runtime foundation docs define current storage/profile persistence and diagnostics boundaries; this feature adds management write APIs plus the minimal SQLite vector execution evolution required to honor an independently bound `vectorTable`
- implementation_order_summary: first define corpus/profile API and persistence compatibility, then switch frontend pages and browser flows, then extend SQLite runtime execution for optional `vectorTable`, then run regression validation across runtime boundaries

## Scope Boundary

### Assumptions

- Current approved frontend shape for corpora uses reusable storage targets plus segment bindings; this UI should remain intact.
- Current runtime still reads legacy `storage_binding_json`; this feature evolves that binding into a role-aware compatibility shape while retaining legacy fields for existing corpora.
- `metadataGroups` already exist inside backend-seeded `capabilities_json`; management edits must preserve them even though the current form does not expose them directly.
- Retrieval profile configuration must support multiple corpora in one profile.
- Strategy and policy lists may be served from backend mock/static catalog data in phase 1; runtime registry discovery can evolve later.
- Diagnostics target remains implicit and derived in the corpus form; role-aware storage excludes diagnostics from graph-capability closure and segment-count rules.

### In Scope

- Backend management HTTP routes under `/api/v1/rag` for:
  - corpora list/detail/catalog/verify/create/update
  - profiles list/detail/catalog/verify/create/update
  - filter validation
- Backend DTO projection that translates persistence/runtime records into frontend form-friendly corpus/profile shapes.
- Backend persistence compatibility layer that:
  - stores the new corpus storage topology separately from legacy runtime binding
  - projects supported topology combinations into legacy runtime `storage_binding_json`
  - preserves `metadataGroups`
- SQLite runtime support for optional `vectorTable` across ingest/search/debug/retry/index maintenance when a split corpus binds vector storage independently from `textUnitTable`.
- Frontend corpus storage binding UI support for an independent `fulltext` segment.
- Backend management API support for `fulltext` topology in catalog/detail/verify/create/update payloads.
- Runtime support for optional SQLite `fulltextTable` carrier rows, FTS index maintenance, fulltext search through the carrier, and hydration from canonical `textUnitTable`.
- Runtime repository dispatch based on route role and segment target backend rather than one whole-corpus backend flag.
- Mixed storage runtime support for canonical SQLite `source`/`textUnit` rows with Graph fulltext and/or vector carrier targets.
- Frontend RAG page data loading for corpora and profiles via API instead of local mock persistence.
- Frontend corpus/profile form save and validation flows via API.
- Focused RAG browser coverage that proves the pages now issue RAG management API requests and still behave as designed.

### Out Of Scope

- Replacing existing runtime RAG routes or changing their request/response contract.
- New diagnostics storage editing. Diagnostics stay implicit and SQLite-backed unless a later feature makes diagnostics storage configurable.
- Building a generic backend strategy discovery registry UI.
- Converting RAG data, diagnostics, or workbench tabs from mock/demo data in this slice.

### Constraints

- Always use Makefile commands first for build/test/cleanup.
- API responses must keep the existing `{ code, message, data }` envelope used by `requestApi`.
- Management changes must not silently drop existing backend-seeded fields such as `metadataGroups`.
- Role-aware compatibility must reject unsupported storage combinations rather than silently persisting unusable runtime config.
- Existing mock-mode UX should remain available only where the app-level data-source environment still selects mock; API mode becomes the real path for RAG management pages.

## Feature Model

### Corpus Management Shape

Management-side `Corpus` detail is a form DTO with:

- identity: `corpusId`, `displayName`, `description`, `status`
- capabilities: `fulltext`, `vector`, `graph`, `reserved`
- metadata groups: preserved read/write companion data from backend capability storage
- filter schema: logical filter fields, value types, and supported operators
- grouped field mapping: `source` and `textUnit`
- management storage topology: target-first `targets[] + bindings{}`
- runtime projection summary: compatibility status, projected legacy binding, and validation issues

The management topology is authoritative for the form. The projected runtime binding exists only for compatibility with the current runtime implementation.

### Profile Management Shape

Management-side `Profile` detail is a form DTO separated by profile type:

- ingest profile:
  - identity/status/description
  - primary corpus
  - stage rows with `enabled`, `strategyId`, and optional stage config
  - source input policy
  - embedding/rerank-related fields already present in the form
- retrieval profile:
  - identity/status/description
  - `primaryCorpusId`, `corpusIds[]`, `requiredSourceVersion`
  - route rows with corpus, route type, strategy id, and enabled flag
  - filter rows
  - fusion/dedup/topic/graph/context policies

### Compatibility Projection Rules

Phase 1.1 accepted corpus storage topology that can be projected into the current runtime model:

- SQLite split: `source -> sqlite(A)` and `textUnit/vector -> sqlite(B)`
- SQLite split with independent vector target: `source -> sqlite(A)`, `textUnit -> sqlite(B)`, and `vector -> sqlite(C)` are accepted in management storage and projected into legacy binding with an optional `vectorTable`
- SQLite embedded source: `source/textUnit/vector -> sqlite(A)`
- Graph single node: `source/textUnit/vector -> graph(A)`

Current runtime compatibility semantics after the previous SQLite vector evolution are:

- runtime management storage may preserve distinct SQLite `source`, `textUnit`, and `vector` bindings
- legacy runtime binding now carries `sourceTable`, `textUnitTable`, and optional `vectorTable`
- current SQLite runtime ingest/search/debug/retry execution reads `vectorTable` when present, creates the required vector business table plus vec sidecar storage on demand, and joins vector hits back to `textUnitTable` through `text_unit_code`

The current role-aware storage evolution extends the projection rules:

- `fulltext` is a first-class binding key; if an old payload omits it, management and runtime derive it from `textUnit`.
- legacy `storage_binding_json` may include `fulltextTable`, `fulltextNamespace`, and `fulltextNodeLabel` in addition to the existing source/textUnit/vector fields.
- SQLite fulltext carriers behave like vector carriers: they store `text_unit_code` plus source/scope identity and a searchable text payload, then join back to `textUnitTable` for canonical hydration.
- Graph fulltext/vector carriers are retriever targets; they may be used without forcing `source` and `textUnit` master data into Graph.
- `capabilities.graph=true` requires at least one non-diagnostics segment target with backend `GRAPH`; `capabilities.graph=false` allows Graph targets to be configured but not required.

The role-aware runtime still rejects:

- missing `source`, `textUnit`, `fulltext`, or `vector` binding after fallback normalization.
- SQLite targets without `TABLE` layout and table name.
- Graph targets without `GRAPH_SINGLE_NODE` layout, namespace, and nodeLabel.
- route/backend mismatches where a route asks for an unsupported retriever strategy for the configured segment backend.
- explicit diagnostics-only storage editing.

## Page And Module Surface

### Pages

- `/rag?tab=corpora`
- `/rag/corpora/new`
- `/rag/corpora/:corpusId/edit`
- `/rag?tab=profiles`
- `/rag/profiles/ingest/new`
- `/rag/profiles/retrieval/new`

### Backend Modules

- Backend:
  - `tron-backend/src/adapters/inbound/http/rag/*`
  - `tron-backend/src/application/rag/*`
  - `tron-backend/src/domain/rag/*`
  - `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
  - `tron-backend/migrations/*rag*`

### Frontend Modules

- Frontend:
  - `tron-frontend/src/domain/rag/**/*`
  - `tron-frontend/src/shared/api/serviceConfig.ts`

## Repository Findings

- Current backend already exposes runtime RAG routes for ingest/search/debug/retry, but no management CRUD routes.
- `RagConfigService` already provides list/get/validate-style building blocks for corpora, profiles, and filters.
- Current backend persistence only understands legacy `storage_binding_json`.
- Current frontend RAG pages and forms are already implemented, but they still read/write browser-local mock data under `tron-frontend/src/domain/rag/mock/data.ts`.
- Existing frontend service infrastructure expects domain-specific API wrappers built on `requestApi`, and can also use `useServiceQuery` once a service domain is declared.
- Existing RAG mock/profile design is already close to backend strategy/stage semantics, but it still needs DTO normalization in both directions.

## Open Decisions

- None.
