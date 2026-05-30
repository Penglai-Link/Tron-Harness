# Feature Spec: RAG 后端底座 - Corpus 存储与 Profile 基础

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: waiting_human_decision

本稿选择 PRD 的第一个可交付切片：`Corpus 存储与 Profile 基础`。该切片只建立 RAG 的 Corpus、Profile、filterSchema、strategy registry、CorpusTableBinding 和 SQLite adapter 边界，给后续 ingest、search、领域字段、debug、retry 和 trace 提供稳定地基。

本稿默认第一阶段新建 `docs/features/rag-corpus-storage-profile-foundation/`，不复用 Graph、Memory 或 Project Doc 的已有 workspace。Graph 相关能力保持边界外；`code-graph`、`GRAPH` retriever 和 SummaryTree 只能作为不可执行扩展位出现。

## Feature Identity And Context

- feature_slug: rag-corpus-storage-profile-foundation
- feature_name: RAG 后端底座 - Corpus 存储与 Profile 基础
- status: draft
- owner: langtaoxin
- source_request: 基于 ONES《RAG MVP 设计方案》和已批准 PRD，先落地第一阶段 RAG 后端底座的 Corpus 存储与 Profile 基础。
- main_agent: current-thread lead agent
- dependency_features: `workspace-management`, `model-management`, `common-event-bus`
- dependency_contracts: `tron-backend/src/adapters/outbound/persistence/sqlite.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`, `tron-backend/src/response.rs`, `tron-backend/src/app.rs`, `tron-backend/src/application/llm/service.rs`, `tron-backend/src/shared/event_bus`

## Requirement

- user_story: 作为 RAG 后续 ingest/search 的后端调用方，我需要先有稳定的 Corpus 存储、Profile 配置、filterSchema 和 adapter 边界，这样 Memory 与 Project Doc 可以在后续切片中共用同一套知识存储和检索基础。
- intent: 建立 RAG 模块的第一层后端地基，避免后续写入、检索、诊断能力各自发明表结构、Profile 语义和字段过滤规则。
- outcome: 后端具备 RAG Corpus/Profile 的 domain/application/persistence 骨架、SQLite schema、内置配置加载和只读查询边界；后续切片可以直接在这些契约上实现 ingest、search 和领域字段。

## Product Source Slices

- brainstorming_source: `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#已确认范围`
- prd_feature_headings: `Module: RAG 后端底座`, `Feature: RAG 后端底座 - Corpus 存储与 Profile 基础`
- prd_page_headings: `Page: Backend API - RAG Debug`
- prd_rule_headings: `Rule: RAG 后端底座 - Corpus 边界`, `Rule: RAG 后端底座 - Profile 与 Registry 边界`, `Rule: RAG 后端底座 - Filter Schema`
- prd_exception_headings: `Exception: RAG 后端底座 - Profile 缺失或策略未注册`, `Exception: RAG 后端底座 - 非法过滤字段`
- prd_acceptance_headings: `Acceptance: RAG 后端底座 - 第一阶段范围`, `Acceptance: RAG 后端底座 - Scope 与访问边界`

## Scope Boundary

### Assumptions

- 第一阶段真实 Corpus 只有 `memory-card` 和 `project-doc`。
- `code-graph`、`GRAPH` retriever 和 SummaryTree 可作为枚举或 Profile 扩展位存在，但不能被启用执行。
- 本切片不执行真实 ingest DAG，也不执行真实 search fan-out。
- 本切片可以定义后续 ingest/search 需要的 Source、Document、TextUnit、FTS、Vector、Profile、warnings、trace 表边界，但只要求基础 repository/config 查询与校验可用。
- Profile 初始配置以内置 seed 或 migration 形式落入 SQLite，Rust registry 只声明有限策略 ID 与能力元数据。

### In Scope

- 新增 RAG bounded context 的 domain/application/persistence 基础结构。
- 定义 `CorpusId`、`CorpusProfile`、`IngestProfile`、`RetrievalProfile`、`CorpusTableBinding`、`FilterSchema`、`StrategyRegistry` 等核心模型。
- 新增 SQLite migration，建立 RAG 第一阶段所需基础表和索引。
- 为 `memory-card` 和 `project-doc` 建立独立 Source/TextUnit/FTS/Vector 相关表的第一版结构。
- 建立 Profile 表、CorpusTableBinding 表、filterSchema 存储字段和内置 Profile seed。
- 建立只读 repository/application 方法，用于列出 Profile、读取 Profile、读取 Corpus binding、校验 filter 字段。
- 保持统一 response 和 HTTP debug 入口的设计边界，但真实 debug API 可在后续诊断切片完成。

### Out Of Scope

- 不实现 `POST /api/v1/rag/ingest` 的完整写入行为。
- 不实现 `POST /api/v1/rag/search` 的检索编排。
- 不调用 `LlmRuntimeService` 生成 embedding。
- 不发布 RAG EventBus 生命周期事件。
- 不实现 indexing retry、search trace TTL 清理或 Context Pack。
- 不接入 `GraphService`。
- 不实现 Memory 治理状态流转。
- 不实现 Profile 管理 UI。

### Constraints

- 文档和代码命名必须使用 code 语义：`userCode`、`agentCode`、`workspaceCode`；数据库隔离字段使用 `scope_type` 和 `scope_code`。
- SQLite 是第一版实现选择，不是架构边界；application 层不能绑定 SQLite 细节。
- Profile 只能引用已注册 strategy ID，不能包含可执行用户代码。
- filterSchema 以逻辑字段映射物理列；未声明字段不能参与过滤。
- API 响应必须继续使用 `ApiResponse<T>` 的 `{ code, message, data }` 结构。
- 后续实现必须优先使用 Makefile 中的构建和测试命令。

## Feature Model

### 核心对象

`CorpusProfile` 描述一个知识域的基础能力、可用表绑定、filterSchema 和默认状态。第一阶段只启用 `memory-card` 与 `project-doc`，`code-graph` 保留为 disabled/reserved。

`CorpusTableBinding` 连接逻辑 Corpus 与物理表名。它记录 Source、Document、TextUnit、FTS、Vector、SummaryTree、GraphNode、GraphEdge 等表名字段；第一阶段对未启用能力可为空或标记 unsupported。

`FilterSchema` 定义调用方可使用的逻辑过滤字段、物理列、值类型、可用操作和适用 Corpus。过滤校验只认 schema 中声明的字段。

`IngestProfile` 描述后续写入 DAG、Stage strategy ID、stage config 和 degrade policy。本切片只保证配置可存储、读取和校验 strategy 引用，不执行 DAG。

`RetrievalProfile` 描述后续检索 routes、filters、fusion、dedup、rerank、hydrate、context 和 degrade policy。本切片只保证配置可存储、读取和校验 retriever/policy 引用，不执行检索。

`StrategyRegistry` 是 Rust 代码中的有限注册表，记录允许出现在 Profile 中的 stage、retriever 和 policy ID。数据库 Profile 引用 registry ID，registry 决定该 ID 在本阶段是 enabled、reserved 还是 unsupported。

### Corpus 状态

- `memory-card`: enabled，第一阶段真实 Corpus。
- `project-doc`: enabled，第一阶段真实 Corpus。
- `code-graph`: reserved，不能用于真实写入或检索。

### Profile 状态

- enabled: 可被后续对应能力使用。
- reserved: 可被读取和展示，但不能执行。
- disabled: 配置存在但不可使用。

### Filter 字段校验

过滤字段校验发生在 application 层。调用方传入逻辑字段后，RAG 根据 Corpus 的 filterSchema 找到物理列和类型规则。未声明字段、类型不匹配字段、跨 Corpus 字段都应返回明确错误。

### 边界规则

本切片只建立基础能力，不定义业务执行成功。后续 ingest/search 切片必须复用这里的 Corpus、Profile、binding、filterSchema 和 registry，不得绕过它们重新定义并行契约。

## Page And Module Surface

UI: Not applicable.

### 后端模块

- `tron-backend/src/domain/rag.rs`
- `tron-backend/src/domain/rag/*`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/*`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `migrations/*_create_rag_foundation.sql`

### 后续 API 边界

本切片可为只读 debug API 准备 application 方法，但不要求完成 HTTP routes。后续诊断切片接入时，应复用本切片的 service/repository 方法，并继续使用 `ApiResponse<T>`。

### 数据库区域

RAG 表应通过独立 migration 创建。第一阶段至少覆盖 Profile、Corpus binding、warnings/trace 基础表，以及 `memory-card` 与 `project-doc` 的 Source/TextUnit/FTS/Vector 相关表边界。具体字段和索引在 `spec/database-design.md` 中定义。

### 共享库与外部服务

本切片不调用外部模型服务。`LlmRuntimeService` 只作为后续 embedding/rerank 的依赖边界被 Profile 字段预留。EventBus 只作为后续生命周期事件边界，不在本切片发布事件。

## Repository Findings

- `tron-backend/src/app.rs` 统一构造 `AppState` 和各 application service，是后续注入 `RagService` 的位置。
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs` 通过 `mod` 加 `pub use` 暴露 SQLite repository，本切片应沿用同一导出模式。
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs` 通过 `include_str!` 注册 SQL migration，本切片应新增 RAG migration 常量并由 RAG repository 执行。
- 现有 repository 通常提供 `connect(database_url)` 并在构造时执行相关 migration，本切片应沿用该模式。
- `tron-backend/src/response.rs` 已有统一 `ApiResponse<T>`，后续 HTTP debug/search/ingest 不需要新响应包装。
- `tron-backend/src/application/llm/service.rs` 已有 `LlmRuntimeService`，但本切片不执行 embedding。
- `tron-backend/src/shared/event_bus` 已有 `EventBusHandle`，但本切片不发布生命周期事件。
- Graph 相关 workspace 与代码已经存在，但 PRD 明确 RAG 第一阶段不接 `GraphService`。
- Strong feature dependencies: `workspace-management` 提供 workspace 概念和访问上下文，`model-management` 与 `LlmRuntimeService` 是后续 embedding/rerank 依赖，`common-event-bus` 是后续 RAG 事件依赖。本 feature 当前只需要保留这些边界，不直接实现其业务。

## Open Decisions

- RAG debug HTTP 是否随本 feature 一并提供最小只读路由；signal: PRD 第一个 feature 引用了 `Page: Backend API - RAG Debug`，但诊断、重试、Trace 与事件是第五个 feature；modules: inbound HTTP RAG routes、RagService；target: `tron-backend/src/adapters/inbound/http/rag`; recommendation: defer to diagnostics feature, this feature only提供 service 查询边界; decision: waiting_human_decision.
- RAG migration 是否一次性创建 warnings 和 trace 表；signal: 后续诊断 feature 依赖这些表，但当前 feature 目标是存储基础；modules: migrations、SQLite repository；target: RAG foundation migration; recommendation: create minimal table shells now, behavior later; decision: waiting_human_decision.
