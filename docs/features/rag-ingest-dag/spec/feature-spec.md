# Feature Spec: RAG 后端底座 - 同步写入 DAG

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

本稿选择已批准 RAG PRD 推荐顺序中的第三个可交付切片：`同步写入 DAG`。前两阶段已经完成 Corpus/Profile/storage binding/filterSchema 基础，以及 Memory 与 Project Doc 领域字段。本阶段让 RAG 第一次具备真实写入能力，为后续 search/context pack 准备可检索数据。

本切片聚焦 ingest 链路：校验请求、加载 Ingest Profile、编译有限 DAG、执行 source/parse/normalize/split/persist/fulltextIndex/vectorIndex，并返回写入结果。它不实现 search、debug/retry、SummaryTree、Graph、Profile 管理 UI 或 Memory 治理状态流转。

## Feature Identity And Context

- feature_slug: rag-ingest-dag
- feature_name: RAG 后端底座 - 同步写入 DAG
- status: approved_with_defaults
- owner: langtaoxin
- source_request: 继续 RAG 后端底座下一阶段，基于已批准 PRD 和已完成的 Corpus/Profile、领域字段能力实现写入链路。
- main_agent: current-thread lead agent
- dependency_features: `rag-corpus-storage-profile-foundation`, `rag-domain-fields`
- dependency_contracts: `tron-backend/src/application/rag/service.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`, `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`, `tron-backend/src/application/llm/service.rs`

## Requirement

- user_story: 作为 Memory 模块、Project Doc 模块和后续 RAG search 调用方，我需要先把文本内容稳定写入 RAG Source、TextUnit、全文字段和向量字段，这样后续检索可以基于同一套 Corpus/Profile 契约执行。
- intent: 用同步、有限、可校验的 DAG 实现 Memory 与 Project Doc 的第一版 ingest。
- outcome: 调用方可以提交 `memory-card` 或 `project-doc` 写入请求，RAG 校验 scope 和 Profile 后持久化 Source/TextUnit，执行全文与向量索引，并返回统一写入结果和 warnings。

## Product Source Slices

- brainstorming_source: `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#第一阶段目标`
- prd_feature_headings: `Feature: RAG 后端底座 - 同步写入 DAG`
- prd_page_headings: `Page: Backend API - RAG Ingest`
- prd_rule_headings: `Rule: RAG 后端底座 - 写入 Scope`, `Rule: RAG 后端底座 - Source 版本幂等`, `Rule: RAG 后端底座 - Embedding 降级`, `Rule: RAG 后端底座 - Filter Schema`
- prd_exception_headings: `Exception: RAG 后端底座 - DAG 配置非法`, `Exception: RAG 后端底座 - Embedding 失败`, `Exception: RAG 后端底座 - 重复写入同版本`
- prd_acceptance_headings: `Acceptance: RAG 后端底座 - 写入链路`, `Acceptance: RAG 后端底座 - Embedding 与 Rerank`, `Acceptance: RAG 后端底座 - Scope 与访问边界`

## Scope Boundary

### Assumptions

- 当前 `master` 已包含 RAG foundation 与 domain fields 两个阶段。
- 第一版只支持 `memory-card` 和 `project-doc` 写入；`code-graph` 继续 reserved。
- 写入请求显式传入 `sourceCode` 和 `sourceVersion`，没有 `latest` 语义。
- 写入不应用空间继承，target scope 必须直接匹配当前 `userCode`、`agentCode` 或 `workspaceCode`。
- 第一版 parser 只支持 plain text 和 Markdown；DOCX、PDF、代码图谱 payload、SummaryTree 都不在本切片。
- 向量能力复用 `LlmRuntimeService`，RAG 不创建独立模型 client，也不复用 Graph embedding 路径。

### In Scope

- 新增 RAG ingest 应用服务和必要 domain models。
- 新增 `POST /api/v1/rag/ingest` 后端 JSON API，并使用统一响应体。
- 加载并校验 Ingest Profile：Corpus、固定策略 ID、DAG、必需 stage、可执行 strategy。
- 编译同步 DAG，校验无环、stage 已注册、artifact 输入输出兼容、persist stage 存在。
- 支持 `RAW_SOURCE`、`SOURCE_RECORD`、`PARSED_DOCUMENT`、`NORMALIZED_DOCUMENT`、`TEXT_UNIT_BATCH`、`PERSIST_RESULT` 等最小 artifacts。
- 支持 plain text 和 Markdown normalize/split，产出 TextUnit。
- 持久化 Memory 与 Project Doc Source/TextUnit，更新全文字段和向量字段。
- 调用 `LlmRuntimeService::embedding_call` 写入 embedding metadata 和 embedding blob。
- Project Doc 向量失败可 active with warnings；Memory 向量失败不可普通可见。
- 同一 corpus/scope/sourceCode/sourceVersion 重试写入保持 upsert 幂等。

### Out Of Scope

- 不实现 `POST /api/v1/rag/search`。
- 不实现检索编排、CandidateHit、fusion、dedup、rerank、hydrate 或 context pack。
- 不实现 debug API、index retry API、search trace 和 EventBus 事件。
- 不实现 SummaryTree、GRAPH retriever、code-graph 写入或 GraphService 集成。
- 不实现 Memory 审核、冲突解决、质量门禁、版本晋级或注入策略。
- 不实现 Profile 管理 UI 或 frontend 页面。
- 不支持 DOCX、PDF、多媒体、外部文档抓取或 repository 扫描。

### Constraints

- 必须优先使用 Makefile 命令执行 backend build/test。
- HTTP API 必须使用项目统一响应体格式。
- 数据库继续使用 SQLite；全文和向量字段仍在 Corpus TextUnit 表内，不新增独立 fulltext/vector 表。
- RAG service 层只能依赖 application ports，不能把 SQLite 类型泄漏到 API contract。
- warning 不能保存 provider 原始响应、API key、embedding 原文之外的敏感 payload。
- 本阶段新增 route 时不能暗示 search/debug/retry 可用。

## Feature Model

### Ingest Request

- `corpusId`: `memory-card` 或 `project-doc`。
- `ingestProfileId`: 内置 Ingest Profile ID。
- `workspaceCode`: 当前 workspace code，用于 HTTP access guard 和 SPACE scope。
- `accessContext`: `userCode`、`agentCode`、`workspaceCode`。
- `targetScope`: `scopeType` 与 `scopeCode`。
- `sourceCode`: 调用方稳定 source code。
- `sourceVersion`: 调用方稳定 source version。
- `sourceType`: 调用方来源类型。
- `payload`: plain text 或 Markdown 内容，以及 Memory/Project Doc 领域字段。

### Ingest Profile

第一版内置两个 enabled Ingest Profile：

- `memory-card-ingest-v1`
- `project-doc-ingest-v1`

Profile 只能引用已注册 stage strategy，不能执行用户自定义代码。第一版 DAG 固定为：

```text
source -> parse -> normalize -> split -> persist -> fulltextIndex -> vectorIndex
```

### Stage Registry

第一版 stage strategy：

- `UPSERT_SOURCE_BY_SOURCE_VERSION`
- `PLAIN_TEXT_PARSER`
- `MARKDOWN_TEXT_PARSER`
- `DEFAULT_TEXT_NORMALIZER`
- `SINGLE_TEXT_UNIT`
- `MARKDOWN_HEADING_SPLITTER`
- `PERSIST_MEMORY_RAG_RECORDS`
- `PERSIST_PROJECT_DOC_RECORDS`
- `SQLITE_TEXT_FIELD_INDEXER`
- `LLM_RUNTIME_VECTOR_INDEXER`

`SQLITE_TEXT_FIELD_INDEXER` 不创建外部 FTS 表；它把可检索文本写入当前 Corpus TextUnit 的 `search_text` 和 `fulltext_status` 字段。

### Artifact Bundle

运行时只允许 stage 读写声明过的 artifact type。Stage 不依赖固定上游 stage 名称，只依赖 artifact type。最小 artifact：

- `RAW_SOURCE`
- `SOURCE_RECORD`
- `PARSED_DOCUMENT`
- `NORMALIZED_DOCUMENT`
- `TEXT_UNIT_BATCH`
- `PERSIST_RESULT`
- `INDEX_WARNING`

### Source State

- 写入开始时 Source 进入 `PENDING`。
- 必需 stage 成功后 Source 进入 `ACTIVE`。
- 必需 stage 失败后 Source 进入 `FAILED`。
- Project Doc 向量失败但全文成功时 Source 可进入 `ACTIVE`，并持久化 warning。
- Memory 向量失败时 Source 进入 `FAILED`。

### TextUnit

Plain text 默认产出一个 `LEAF` TextUnit。Markdown 按 heading 和段落生成多个 `LEAF` TextUnit。TextUnit 稳定键使用 `sourceCode + sourceVersion + ordinal`，重试写入时 upsert，不重复插入。

### Warning

Warning 以 `INDEXING_WARNING` 形式写入 `rag_diagnostics`，并在 ingest response 中返回摘要。Warning 不保存 provider 原始响应、密钥或 embedding blob。

## Page And Module Surface

### Backend API - RAG Ingest

- route: `POST /api/v1/rag/ingest`
- request: JSON body
- response: 项目统一响应体
- auth: 复用现有认证和 workspace access guard
- success state: active 或 active with warnings
- failure state: profile invalid、scope invalid、payload invalid、stage failed、embedding failed

### Rust Modules

- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/domain/rag/ingest.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/inbound/http/rag/*`
- `tron-backend/tests/rag_ingest_contracts.rs`

## Repository Findings

- 当前 `RagService` 已提供 Corpus/Profile 读取、Profile 校验和 filter validation。
- 当前 `SqliteRagRepository` 已创建 Memory/Project Doc Source 和 TextUnit 表，并支持幂等 schema evolution。
- TextUnit 表已包含 `search_text`、`fulltext_status`、`embedding_model_code`、`embedding_version`、`embedding_text_hash`、`embedding_dimension`、`embedding`、`embedding_status`。
- 当前 RAG 没有 ingest/search HTTP route，也没有写入 Source/TextUnit 的 repository port。
- 当前 `LlmRuntimeService` 已提供 embedding 能力；RAG 应复用它而不是新建模型 client。
- 当前 graph 服务可用但不属于本阶段，RAG ingest 不应接入 `GraphService`。

## Open Decisions

- None.
