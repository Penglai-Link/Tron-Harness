# Feature Spec: RAG 后端底座 - 诊断、重试、Trace 与事件

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

本稿选择已批准 RAG PRD 推荐顺序中的第五个可交付切片：`诊断、重试、Trace 与事件`。前置的 Corpus/Profile 基础、Memory/Project Doc 领域字段、同步写入 DAG 和检索 Context Pack 已合入 `master`，当前阶段补齐后端排障、索引恢复和生命周期观测闭环。

本切片聚焦运行诊断：只读 debug API 查询 Profile、Source、TextUnit 和 warnings；retry API 基于已持久化 TextUnit 重建 fulltext/vector 索引；search trace 仅保存轻量诊断数据并按 7 天 TTL 管理；EventBus 事件只发布非正文、非向量、非密钥 payload。它不实现管理后台、Profile UI、SummaryTree、GRAPH retriever、GraphService 集成、代码图谱或 Memory 治理状态流转。

## Feature Decomposition Summary

- feature_slug: rag-diagnostics-retry-trace-events
- feature_name: RAG 后端底座 - 诊断、重试、Trace 与事件
- delivery_type: new_feature
- target_workspace: `docs/features/rag-diagnostics-retry-trace-events`
- goal: 让后端开发者、平台运维和调用方可以定位 RAG 写入/检索问题，并在模型或索引失败后安全重试索引阶段。
- in_scope: read-only debug API、single-source indexing retry API、warnings 查询、search trace 持久化与 TTL、RAG EventBus 生命周期事件、安全 payload 边界。
- deferred_or_out_of_scope: 管理后台、debug UI、retry UI、Profile 管理 UI、SummaryTree、GRAPH、code-graph、GraphService、agentic retrieval、Memory 治理流转。
- dependency_features: `rag-corpus-storage-profile-foundation`, `rag-domain-fields`, `rag-ingest-dag`, `rag-search-context-pack`
- source_prd_headings: `Feature: RAG 后端底座 - 诊断、重试、Trace 与事件`, `Page: Backend API - RAG Debug`, `Page: Backend API - RAG Indexing Retry`, `Rule: RAG 后端底座 - Warnings 持久化`, `Rule: RAG 后端底座 - Trace 隐私边界`, `Rule: RAG 后端底座 - Retry 不重跑解析`, `Exception: RAG 后端底座 - Retry Source 不存在`, `Exception: RAG 后端底座 - Event payload 越界`, `Acceptance: RAG 后端底座 - 诊断与安全边界`
- recommended_order: PRD 推荐顺序第 5 步；前 4 个依赖 feature 已完成。

共享能力盘点：

- Workspace Access Guard: ingest/search 已使用同一 RAG HTTP guard，本阶段 debug/retry 必须复用同一判断。
- SQLite Persistence Adapter: 复用已存在的 `rag_diagnostics` 表；不新增主数据表。
- LlmRuntimeService: retry vector 阶段复用 embedding runtime，不新建模型 client。
- EventBus: 使用项目已有 `EventBusHandle` 和 `BusEventDraft`，事件 payload 遵守现有 secret/size 校验。
- 统一响应体: debug/retry/search trace 相关 HTTP 响应继续返回 `{ code, message, data }`。
- code-based identifiers: `userCode`、`agentCode`、`workspaceCode`、`sourceCode`、`sourceVersion` 贯穿 API、diagnostics 和事件。

依赖图：

- `rag-corpus-storage-profile-foundation` -> `rag-diagnostics-retry-trace-events`
- `rag-domain-fields` -> `rag-diagnostics-retry-trace-events`
- `rag-ingest-dag` -> `rag-diagnostics-retry-trace-events`
- `rag-search-context-pack` -> `rag-diagnostics-retry-trace-events`

## Feature Identity And Context

- status: approved_with_defaults
- owner: langtaoxin
- source_request: 上一阶段 RAG 检索 Context Pack 已验收，继续 PRD 推荐顺序中的最后一个第一阶段切片。
- main_agent: current-thread lead agent
- dependency_features: `rag-corpus-storage-profile-foundation`, `rag-domain-fields`, `rag-ingest-dag`, `rag-search-context-pack`
- dependency_contracts: `tron-backend/src/application/rag/ingest.rs`, `tron-backend/src/application/rag/search.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`, `tron-backend/src/shared/event_bus`

## Requirement

- user_story: 作为后端开发者、平台运维和 RAG 调用方，我需要只读查看 RAG Profile/Source/TextUnit/warnings，按 Source 重试索引阶段，并获得安全 trace 与生命周期事件，以便定位问题并恢复索引。
- intent: 在核心 ingest/search 链路可用后，补齐第一阶段运维闭环，同时保持正文、embedding、provider 原始响应和密钥不进入 trace 或事件。
- outcome: 调用方可以通过 debug API 查看诊断数据，通过 retry API 重建 fulltext/vector 索引；search 可以按请求开关写入轻量 trace；RAG 关键生命周期事件进入 EventBus。

## Product Source Slices

- brainstorming_source: `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#已确认范围`
- prd_feature_headings: `Feature: RAG 后端底座 - 诊断、重试、Trace 与事件`
- prd_page_headings: `Page: Backend API - RAG Debug`, `Page: Backend API - RAG Indexing Retry`
- prd_interaction_headings: `Interaction: RAG Debug - 查看诊断数据`, `Interaction: RAG Indexing Retry - 重试单 Source 索引`
- prd_rule_headings: `Rule: RAG 后端底座 - Warnings 持久化`, `Rule: RAG 后端底座 - Trace 隐私边界`, `Rule: RAG 后端底座 - Retry 不重跑解析`
- prd_exception_headings: `Exception: RAG 后端底座 - Retry Source 不存在`, `Exception: RAG 后端底座 - Trace TTL 到期`, `Exception: RAG 后端底座 - Event payload 越界`
- prd_acceptance_headings: `Acceptance: RAG 后端底座 - 诊断与安全边界`, `Acceptance: RAG 后端底座 - Scope 与访问边界`

## Scope Boundary

### Assumptions

- 当前 `master` 已包含 RAG foundation、domain fields、ingest DAG 和 search Context Pack。
- `rag_diagnostics` 表已存在，支持 `INDEXING_WARNING` 与 `SEARCH_TRACE`。
- 第一版 debug/retry 只支持 `memory-card` 和 `project-doc`；`code-graph` 继续 reserved。
- retry 只读取已持久化 Source/TextUnit，不接收新正文，不重新执行 parse、normalize、split。
- search trace 由请求开关启用，默认不写入。
- EventBus 发布失败不回滚主流程，但应在返回 warnings 或日志中保留可诊断摘要。

### In Scope

- 新增 read-only debug API，用于查看 Profile、Source、TextUnits 和 warnings。
- 新增 single-source indexing retry API，用于重建 `fulltextIndex`、`vectorIndex` 或两者。
- retry fulltext 基于已持久化 TextUnit `text` 重建 `search_text` 和 `fulltext_status`。
- retry vector 复用 `LlmRuntimeService::embedding_call`，更新 TextUnit embedding 字段和 `embedding_status`。
- retry warnings 写入 `rag_diagnostics`，不保存 provider 原始响应或 API key。
- search trace 写入 `rag_diagnostics`，保存 query text、route counts、timings、warnings、candidate references、result count 和 token usage，TTL 为 7 天。
- debug API 可查询 `INDEXING_WARNING` 和 `SEARCH_TRACE` 摘要。
- RAG ingest/search/retry/stage/route 发布 EventBus 生命周期事件。
- Event payload 只包含 codes、status、counts、stage/route 名称、warning code 和 traceCode，不包含正文、Context Pack body、embedding、provider 原始响应或 API keys。
- 更新上一阶段边界测试，允许本阶段新增 debug/retry/trace/EventBus，但继续禁止 Graph、SummaryTree、code-graph 和 Memory 治理流转。

### Out Of Scope

- 不实现管理后台、debug UI、retry UI 或 Profile 管理 UI。
- 不实现 SummaryTree、GRAPH retriever、code-graph 或 GraphService 集成。
- 不实现 retry parse、normalize、split 或重新读取原始 payload。
- 不实现复杂多跳检索、Agentic retrieval 或外部队列重放。
- 不做完整权限、角色、字段权限或内容脱敏。
- 不替换 SQLite，不接 pgvector、Qdrant、Milvus 或 Elasticsearch。

### Constraints

- 必须优先使用 Makefile 命令执行 backend build/test。
- HTTP API 必须使用项目统一响应体格式。
- Debug 与 retry 必须复用 RAG HTTP workspace access guard。
- Debug 与 retry 不绕过 RAG scope；Source 查询必须限定 `scopeType` 和 `scopeCode`。
- Search trace TTL 固定为 7 天。
- Trace/Event payload 禁止保存正文、Context Pack body、embedding、provider 原始响应和 API keys。
- EventBus 发布失败不能导致已成功的 ingest/search/retry 主流程回滚。

## Feature Model

### Debug Profile Query

- `workspaceCode`: 当前 workspace code。
- `profileId`: Profile ID。
- output: Profile 基础信息、Corpus 列表、status、requiredSourceVersion、filters、degradePolicy、配置摘要。

### Debug Source Query

- `workspaceCode`: 当前 workspace code。
- `corpusId`: `memory-card` 或 `project-doc`。
- `sourceCode`: Source code。
- `sourceVersion`: Source version。
- `scopeType`: `USER`、`AGENT` 或 `SPACE`。
- `scopeCode`: 对应 code。
- `includeTextUnits`: 是否返回 TextUnit 摘要。
- `includeWarnings`: 是否返回 warnings。
- output: Source 状态、领域字段、TextUnit 摘要、warnings 摘要；不返回 embedding BLOB。

### Indexing Retry Request

- `workspaceCode`
- `corpusId`
- `sourceCode`
- `sourceVersion`
- `scopeType`
- `scopeCode`
- `stages`: 只允许 `fulltextIndex`、`vectorIndex`
- `embeddingModelCode`: vector retry 使用；缺省时使用 Profile 或已有 TextUnit embedding model。
- output: retry status、每个 stage 的结果、更新数量、warnings、traceCode。

### Search Trace Record

- `record_type`: `SEARCH_TRACE`
- `record_code`: trace code。
- `profile_id`
- `workspace_code`
- `user_code`
- `agent_code`
- `query_text`
- `payload_json`: route counts、timings、warnings、candidate refs、result count、token usage。
- `expires_at`: `created_at + 7 days`。

### EventBus Payload

允许字段：

- `workspace_code`
- `corpus_id`
- `profile_id`
- `source_code`
- `source_version`
- `scope_type`
- `scope_code`
- `stage_id`
- `route_id`
- `status`
- `warning_code`
- `trace_code`
- `count`

禁止字段：

- Source/TextUnit 正文
- Context Pack body
- embedding 或向量数组
- provider 原始响应
- API key、token、secret

## Page And Module Surface

### Backend API - RAG Debug

- route: `GET /api/v1/rag/debug/profile`
- route: `GET /api/v1/rag/debug/source`
- response: 项目统一响应体
- auth: 复用认证和 workspace access guard
- success state: 返回只读诊断数据
- failure state: workspace denied、profile not found、source not found、scope invalid、corpus unsupported

### Backend API - RAG Indexing Retry

- route: `POST /api/v1/rag/retry/index`
- request: JSON body
- response: 项目统一响应体
- auth: 复用认证和 workspace access guard
- success state: success、partial warnings
- failure state: workspace denied、source not found、text units missing、unsupported stage、embedding failed

### Backend API - RAG Search Trace

- route: `POST /api/v1/rag/search`
- request addition: optional `trace.enabled`
- response addition: optional `traceCode`
- behavior: trace disabled by default；enabled 时写入 `rag_diagnostics`。

### Rust Modules

- `tron-backend/src/application/rag/diagnostics.rs`
- `tron-backend/src/application/rag/retry.rs`
- `tron-backend/src/application/rag/events.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/diagnostics.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/retry.rs`
- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/tests/rag_diagnostics_api.rs`
- `tron-backend/tests/rag_diagnostics_contracts.rs`

## Repository Findings

- 当前 `rag_diagnostics` 表已存在，且 ingest vector degrade 已写入 `INDEXING_WARNING`。
- 当前 RAG HTTP route 已注册 ingest/search，尚未注册 debug/retry。
- 当前 search 明确禁止 trace 持久化和 EventBus，本阶段需要更新这些边界测试。
- 当前 `AppState` 已有 `EventBusHandle`，其他 application service 已使用 `BusEventDraft::audit` 发布事件。
- 当前 TextUnit 表在一行内保存 `text`、`search_text`、`fulltext_status`、embedding 字段和 `embedding_status`，适合 retry 直接更新。
- 当前 `LlmRuntimeService` 已有 embedding 能力，retry vector 应复用。

## Open Decisions

- None.
