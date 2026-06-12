# Feature Spec: RAG 后端底座 - 检索编排与 Context Pack

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

本稿选择已批准 RAG PRD 推荐顺序中的第四个可交付切片：`检索编排与 Context Pack`。前置的 Corpus/Profile 基础、Memory/Project Doc 领域字段和同步写入 DAG 已合入 `master`，当前阶段把已写入的 Source/TextUnit 变成可查询、可 hydrate、可供 Agent 消费的上下文结果。

本切片聚焦 search 链路：校验请求、加载 Retrieval Profile、解析固定 source version、合并 filterSchema 过滤条件、执行 fulltext/vector routes、融合去重、可选 rerank、hydrate active Source/TextUnit，并组装 `RESULT_LIST` 与基础 Context Pack。它不实现 debug API、index retry API、trace 持久化、EventBus 生命周期事件、SummaryTree、GRAPH retriever 或 GraphService 集成。

## Feature Decomposition Summary

- feature_slug: rag-search-context-pack
- feature_name: RAG 后端底座 - 检索编排与 Context Pack
- delivery_type: new_feature
- target_workspace: `docs/features/rag-search-context-pack`
- goal: 让调用方可以通过 RAG search API 检索 Memory 和 Project Doc，并拿到可直接用于 Agent prompt 的结果或 Context Pack。
- in_scope: search API、Retrieval Profile 编译、scope/filter/sourceVersion 约束、fulltext/vector fan-out、fusion/dedup、可选 rerank、hydrate、context assembly。
- deferred_or_out_of_scope: debug、retry、trace、EventBus、SummaryTree、GRAPH、code-graph、Profile UI、复杂多跳检索和 agentic retrieval。
- dependency_features: `rag-corpus-storage-profile-foundation`, `rag-domain-fields`, `rag-ingest-dag`
- source_prd_headings: `Feature: RAG 后端底座 - 检索编排与 Context Pack`, `Page: Backend API - RAG Search`, `Interaction: RAG Search - 提交检索请求`, `Rule: RAG 后端底座 - 检索 Scope`, `Rule: RAG 后端底座 - Version Filter`, `Rule: RAG 后端底座 - CandidateHit 统一结构`, `Exception: RAG 后端底座 - 缺少版本过滤`, `Exception: RAG 后端底座 - 向量 route 不可用`, `Exception: RAG 后端底座 - Hydrate 后 Source 不可见`, `Acceptance: RAG 后端底座 - 检索链路`
- recommended_order: PRD 推荐顺序第 4 步；前 3 个依赖 feature 已完成。

共享能力盘点：

- Workspace Access Guard: ingest 已有局部校验，本阶段应抽出 RAG HTTP 共用 guard 给 search 复用，避免分页或行为漂移。
- SQLite Persistence Adapter: search 只新增读取能力和 retrieval profile seed，继续复用 Source/TextUnit 表中合并后的全文和向量字段。
- LlmRuntimeService: search vector route 使用 embedding，rerank 启用时使用 rerank；不新建模型 client。
- 统一响应体: search 继续返回 `{ code, message, data }`。
- code-based identifiers: `userCode`、`agentCode`、`workspaceCode` 继续贯穿 scope 与 API。

依赖图：

- `rag-corpus-storage-profile-foundation` -> `rag-search-context-pack`
- `rag-domain-fields` -> `rag-search-context-pack`
- `rag-ingest-dag` -> `rag-search-context-pack`
- `rag-search-context-pack` -> 后续 `诊断、重试、Trace 与事件`

## Feature Identity And Context

- status: approved_with_defaults
- owner: langtaoxin
- source_request: 上一阶段 RAG 同步写入 DAG 已验收，继续 PRD 推荐顺序中的下一阶段。
- main_agent: current-thread lead agent
- dependency_features: `rag-corpus-storage-profile-foundation`, `rag-domain-fields`, `rag-ingest-dag`
- dependency_contracts: `tron-backend/src/application/rag/service.rs`, `tron-backend/src/application/rag/ingest.rs`, `tron-backend/src/application/llm/service.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs`

## Requirement

- user_story: 作为 Agent 运行时、Memory 上下文调用方和 Project Doc 问答调用方，我需要通过统一 RAG search API 按 Profile 检索知识，并拿到已按 scope、版本和过滤条件约束过的结果或 Context Pack。
- intent: 在不引入 Graph、SummaryTree 或 trace/retry 的前提下，跑通 Memory 与 Project Doc 的第一版真实检索链路。
- outcome: 调用方可以提交 `profileId`、query、AccessContext、filters、limit、tokenBudget 和 context 策略，RAG 返回去重后的 CandidateHit hydrate 结果或上下文包。

## Product Source Slices

- brainstorming_source: `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#检索`
- prd_feature_headings: `Feature: RAG 后端底座 - 检索编排与 Context Pack`
- prd_page_headings: `Page: Backend API - RAG Search`
- prd_interaction_headings: `Interaction: RAG Search - 提交检索请求`
- prd_rule_headings: `Rule: RAG 后端底座 - 检索 Scope`, `Rule: RAG 后端底座 - Version Filter`, `Rule: RAG 后端底座 - CandidateHit 统一结构`, `Rule: RAG 后端底座 - Embedding 降级`, `Rule: RAG 后端底座 - Filter Schema`
- prd_exception_headings: `Exception: RAG 后端底座 - 缺少版本过滤`, `Exception: RAG 后端底座 - 向量 route 不可用`, `Exception: RAG 后端底座 - Hydrate 后 Source 不可见`
- prd_acceptance_headings: `Acceptance: RAG 后端底座 - 检索链路`, `Acceptance: RAG 后端底座 - Embedding 与 Rerank`, `Acceptance: RAG 后端底座 - Scope 与访问边界`

## Scope Boundary

### Assumptions

- 当前 `master` 已包含 RAG foundation、domain fields 和 ingest DAG。
- 第一版 search 只执行 `memory-card` 与 `project-doc` Corpus；`code-graph` 继续 reserved。
- 检索请求没有 `latest` 语义；请求未传 sourceVersion filter 时，Retrieval Profile 必须配置固定 `required_source_version`。
- 空间继承第一版只按当前 AccessContext 的 USER、AGENT、SPACE 三个直接 scope 查询，不接 Space lineage。
- 向量检索使用 SQLite 内已存 embedding blob 做本地相似度计算，适合 MVP 小数据量。
- Rerank 由 Profile 控制，失败按 Profile degrade policy 处理。

### In Scope

- 新增 `POST /api/v1/rag/search` 后端 JSON API。
- 抽出 RAG HTTP 共用 workspace access guard，供 ingest/search 复用。
- 加载并校验 Retrieval Profile：enabled、Corpus、routes、source version、degrade policy、context policy。
- 合并 Profile filters 和 Request filters，字段必须在 filterSchema 中声明，同字段取交集，不同字段 AND。
- 生成当前 USER、AGENT、SPACE scope filter。
- 支持 FULLTEXT route 和 VECTOR route。
- fulltext route 基于 TextUnit `search_text` 做 SQLite LIKE/MATCH 等本地召回。
- vector route 调用 `LlmRuntimeService::embedding_call` 生成 query embedding，并与 TextUnit embedding 计算相似度。
- 统一输出 CandidateHit，执行 Weighted RRF 或简化等价融合、dedup 和排序。
- 可选调用 `LlmRuntimeService::rerank_call`。
- Hydrate Source/TextUnit，并二次校验 Source 为 `ACTIVE`。
- 支持 `RESULT_LIST`、`TOKEN_BUDGET_PACK`、`MEMORY_CONTEXT_PACK`、`DOC_QA_CONTEXT_PACK`。

### Out Of Scope

- 不实现 read-only debug API。
- 不实现 indexing retry API。
- 不实现 search trace 持久化、Trace TTL 清理或 traceCode。
- 不发布 EventBus 生命周期事件。
- 不实现 SummaryTree route、GRAPH route、HYBRID_TREE、code-graph 或 GraphService 集成。
- 不实现复杂多跳检索、Agentic retrieval 或 sub-question decomposition。
- 不实现 Profile 管理 UI 或前端页面。
- 不做完整权限、角色、字段权限或内容脱敏。

### Constraints

- 必须优先使用 Makefile 命令执行 backend build/test。
- HTTP API 必须使用项目统一响应体格式。
- 数据库继续使用 SQLite；不新增独立全文或向量表。
- Search 服务层不能泄漏 SQLite 细节给 HTTP contract。
- Search 不能保存 hydrate 正文、Context Pack body、embedding、provider 原始响应或 API key 到 trace，因为 trace 不在本阶段。
- 本阶段新增 search route 后，需要更新上一阶段禁止 search route 的边界回归。

## Feature Model

### Search Request

- `profileId`: Retrieval Profile ID，例如 `memory-related-v1` 或 `project-doc-related-v1`。
- `workspaceCode`: 当前 workspace code。
- `query`: 非空检索文本。
- `accessContext`: `userCode`、`agentCode`、`workspaceCode`。
- `filters`: 请求过滤条件，必须在 filterSchema 中声明。
- `limit`: 返回条数上限。
- `tokenBudget`: Context Pack token 上限。
- `context`: 请求输出形态，可覆盖 Profile 默认 context policy 的安全子集。

### Retrieval Profile

第一版复用已 seed 的 Retrieval Profile，并允许补齐配置：

- `memory-related-v1`
- `project-doc-related-v1`

Profile 必须是 enabled，必须提供固定 `required_source_version`，并只能启用第一阶段支持的 route 类型。

### Search Routes

支持 route：

- `FULLTEXT`: 从 Corpus TextUnit `search_text` 召回。
- `VECTOR`: 生成 query embedding 后与 TextUnit embedding 做本地相似度召回。

Reserved route：

- `GRAPH`
- `SUMMARY_TREE`
- `HYBRID_TREE`

### CandidateHit

每条命中统一为 CandidateHit：

- `corpusId`
- `sourceCode`
- `sourceVersion`
- `targetType`
- `targetCode`
- `dedupKey`
- `routeId`
- `rank`
- `score`
- `scopeType`
- `scopeCode`
- `preview`
- `metadata`

### Context Assembly

支持输出：

- `RESULT_LIST`: 命中列表，包含 hydrate 摘要和 metadata。
- `TOKEN_BUDGET_PACK`: 按 tokenBudget 截断的上下文段落。
- `MEMORY_CONTEXT_PACK`: Memory 语义上下文，包含 memory fields。
- `DOC_QA_CONTEXT_PACK`: Project Doc 问答上下文，包含 title、path、headingPath 和片段。

## Page And Module Surface

### Backend API - RAG Search

- route: `POST /api/v1/rag/search`
- request: JSON body
- response: 项目统一响应体
- auth: 复用认证和 workspace access guard
- success state: result list、empty result、partial vector degraded
- failure state: profile invalid、filter invalid、version missing、workspace denied、route unavailable、hydrate source inactive

### Rust Modules

- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_profile.rs`
- `tron-backend/src/application/rag/search_context.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_search_api.rs`

## Repository Findings

- 当前 `RagService` 已提供 Corpus/Profile 读取、Profile validation 和 filter validation。
- 当前 `SqliteRagRepository` 已有 Retrieval Profile seed，`memory-related-v1` 固定 `required_source_version=v1`。
- 当前 `rag_memory_text_units` 与 `rag_project_doc_text_units` 已保存 `search_text`、embedding blob 和 embedding metadata。
- 当前 ingest 已能写入 Memory 与 Project Doc 的 Source/TextUnit，可作为 search 测试前置数据。
- 当前 HTTP RAG route 只注册 ingest；新增 search route 时需要调整 ingest 阶段的边界测试。
- 当前 `LlmRuntimeService` 已有 embedding 与 rerank 能力；search 应复用它。
- 当前 RAG handler 内 workspace access guard 是 ingest 局部逻辑，本阶段应抽出共用 helper。

## Open Decisions

- None.
