# Feature Spec: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

本稿选择 PRD 推荐顺序中的第二个可交付切片：`Memory 与 Project Doc 领域字段`。上一阶段已经完成 RAG Corpus/Profile/storage binding/filterSchema 基础；本阶段先稳定 Memory 和 Project Doc 的领域字段，避免后续 ingest/search 实现后反复改表、改字段映射和过滤规则。

本切片仍然是后端内部能力，不新增 HTTP route，不实现真实 ingest/search，不调用模型、Graph 或 EventBus。它只扩展 RAG 存储 schema、Corpus seed、fieldMapping、filterSchema 和 service 过滤校验，使后续写入与检索可以复用稳定字段边界。

## Feature Identity And Context

- feature_slug: rag-domain-fields
- feature_name: RAG 后端底座 - Memory 与 Project Doc 领域字段
- status: approved_with_defaults
- owner: langtaoxin
- source_request: 基于已批准 RAG PRD 的推荐顺序，在 Corpus/Profile 基础之后落地 Memory 与 Project Doc 的可过滤领域字段。
- main_agent: current-thread lead agent
- dependency_features: `rag-corpus-storage-profile-foundation`
- dependency_contracts: `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/src/application/rag/service.rs`, `tron-backend/src/domain/rag/model.rs`

## Requirement

- user_story: 作为 Memory、Project Doc 和后续 RAG ingest/search 调用方，我需要 RAG 先稳定存储领域字段、字段映射和过滤规则，这样写入、检索和 hydrate 能共用同一套字段契约。
- intent: 在不引入业务状态流转和检索执行的前提下，把 Memory 治理字段与 Project Doc 文档字段提升为可过滤物理列。
- outcome: RAG SQLite schema、Corpus fieldMapping、filterSchema 和 `RagService::validate_filters` 能识别 Memory 与 Project Doc 的核心领域字段；metadata 继续只承载低频扩展。

## Product Source Slices

- brainstorming_source: `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#已确认范围`
- prd_feature_headings: `Feature: RAG 后端底座 - Memory 与 Project Doc 领域字段`
- prd_page_headings: `Page: Backend API - RAG Ingest`, `Page: Backend API - RAG Search`
- prd_rule_headings: `Rule: RAG 后端底座 - Memory 治理字段只存储`, `Rule: RAG 后端底座 - Metadata 使用边界`, `Rule: RAG 后端底座 - Filter Schema`
- prd_exception_headings: `Exception: RAG 后端底座 - 未声明过滤字段`, `Exception: RAG 后端底座 - Metadata 字段误用`
- prd_acceptance_headings: `Acceptance: RAG 后端底座 - 写入链路`, `Acceptance: RAG 后端底座 - 检索链路`, `Acceptance: RAG 后端底座 - Scope 与访问边界`

## Scope Boundary

### Assumptions

- `rag-corpus-storage-profile-foundation` 已经完成，当前 schema 已有 `rag_corpora`、`rag_profiles`、Memory/Project Doc Source 和 TextUnit 表。
- 本阶段只处理 `memory-card` 与 `project-doc`；`code-graph` 继续 reserved。
- Memory 治理字段由 Memory 模块传入和解释；RAG 只存储字段值，不推进审核、冲突、候选、版本或注入状态流转。
- Project Doc 字段用于过滤、排序和 hydrate，不负责文档解析、切分或索引。
- filterSchema 继续使用逻辑字段映射物理列；未声明字段不能参与过滤。
- SQLite schema evolution 必须幂等，因为 repository 初始化会重复执行 migration 逻辑。

### In Scope

- 为 `rag_memory_sources` 添加 Memory 核心治理字段物理列。
- 为 `rag_project_doc_sources` 添加 Project Doc 仓库和模块字段物理列，并复用已有 document/path 字段。
- 更新 `rag_corpora` 内置 seed 的 `filter_schema_json` 和 `field_mapping_json`。
- 扩展 `RagService::validate_filters` 覆盖新增 logical fields 的类型和操作符校验。
- 增加 schema 与 service contract 测试，证明字段存在、配置可读、过滤校验可用。
- 保持 metadata 边界：长尾治理细节、审核原因、质量门禁详情、冲突详情、派生链路详情仍进入 metadata。

### Out Of Scope

- 不实现 `POST /api/v1/rag/ingest`。
- 不实现 `POST /api/v1/rag/search`。
- 不写入 Source 或 TextUnit 业务数据。
- 不实现 Memory 状态流转、候选审核、冲突解决、版本晋级或注入策略。
- 不调用 `LlmRuntimeService`、`GraphService` 或 `EventBusHandle`。
- 不新增 HTTP route、frontend 页面或 Profile 管理 UI。
- 不实现 SummaryTree、GRAPH retriever 或 code-graph 字段。

### Constraints

- 字段命名继续遵循 code 语义：`userCode`、`agentCode`、`workspaceCode`；数据库隔离字段继续使用 `scope_type` 和 `scope_code`。
- 数据库物理列使用 snake_case；service logical fields 使用 camelCase。
- 新增字段必须在 Corpus `filter_schema_json` 和 `field_mapping_json` 中声明后才能用于 filter。
- SQLite schema evolution 不能依赖只执行一次的迁移表；应使用 `column_exists` 风格保护。
- 后续实现必须优先使用 Makefile 中的构建和测试命令。

## Feature Model

### Memory 领域字段

Memory 核心字段存放在 `rag_memory_sources`。本阶段建议提升为物理列的字段包括：

- `memory_type`
- `memory_status`
- `review_status`
- `version`
- `superseded_by`
- `conflict_group`
- `duplicate_group`
- `injectable`
- `importance_score`
- `confidence_score`
- `quality_score`
- `pin_level`
- `expires_at`
- `derived_source_type`
- `derived_source_code`

这些字段只表达调用方传入的事实和治理状态。RAG 不判断这些状态是否应该改变，也不根据这些字段自动写入其他状态。

### Project Doc 领域字段

Project Doc 第一阶段复用已有 `document_code`、`document_type`、`path`、`title`、`body_hash`。本阶段补充仓库与模块字段：

- `repo_code`
- `module_code`
- `module_name`

逻辑字段 `documentType` 映射到既有 `document_type`，不重复新增 `doc_type` 列。

### FilterSchema 与 FieldMapping

`memory-card` filterSchema 增加 `memoryType`、`memoryStatus`、`reviewStatus`、`injectable`、`importanceScore`、`confidenceScore`、`qualityScore`、`pinLevel`、`expiresAt`、`derivedSourceType`、`derivedSourceCode` 等字段。

`project-doc` filterSchema 增加 `repoCode`、`moduleCode`、`moduleName`，并声明已有 `documentCode`、`documentType`、`path`。

数值字段使用 integer 类型，布尔字段使用 boolean 类型，字符串字段使用 string 类型。排序能力不在本阶段执行，但字段类型要先稳定。

### Metadata 边界

metadata 保存低频扩展：review reason、quality gate detail、conflict detail、duplicate detail、derived trace、reviewer code、audit hints、文档标签扩展等。metadata 字段默认不能参与过滤；如果后续需要过滤，必须先通过 schema evolution 提升为物理列并进入 filterSchema。

## Page And Module Surface

UI: Not applicable.

### 后端模块

- `tron-backend/migrations/*_add_rag_domain_fields.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/application/rag/service.rs`
- `tron-backend/tests/rag_foundation_schema.rs`
- `tron-backend/tests/rag_service_contracts.rs`

### 后续 API 边界

本切片不新增 HTTP API。后续 ingest/search API 的 request payload、Profile filters、request filters 和 hydrate 输出必须复用本切片声明的 logical fields。

### 数据库区域

本切片只演进 RAG 既有表，不新增新表。Memory 字段添加到 `rag_memory_sources`；Project Doc 字段添加到 `rag_project_doc_sources`；`rag_corpora` seed 更新 fieldMapping/filterSchema。

## Repository Findings

- 当前 RAG foundation migration 已创建 7 张表，Source/TextUnit 按 Corpus 分表，全文和向量字段位于 TextUnit 表。
- `SqliteRagRepository::connect` 当前只执行 `RAG_FOUNDATION_MIGRATION`，本阶段需要加入可重复执行的 schema evolution 逻辑。
- 项目已有 `model`、`agent`、`chat`、`skill` repository 使用 `PRAGMA table_info` 检查列存在后再 `ALTER TABLE ADD COLUMN` 的模式。
- `RagService::validate_filters` 已经按 Corpus filterSchema 校验字段、操作符和值类型，可以直接扩展 seed 配置和测试覆盖。
- 当前 `RagFilterValueType` 支持 string、integer、boolean；本阶段领域字段可以落在这三类内。
- 当前没有 RAG HTTP route，也没有 RAG ingest/search 实现；本阶段应保持该边界。

## Open Decisions

- 是否新增 `doc_type` 列；signal: Project Doc source 已经有 `document_type`; modules: migration、fieldMapping; target: `rag_project_doc_sources`; recommendation: 复用 `document_type` 并映射 logical field `documentType`; decision: approved_with_defaults.
- Memory score 字段使用 integer 还是 real；signal: 当前 filter validator 只支持 integer、boolean、string; modules: filterSchema、service tests; target: Memory score columns; recommendation: 第一阶段使用 integer 分值，后续如需小数再演进; decision: approved_with_defaults.
- 是否把 Project Doc tags 提升为物理列；signal: tags 通常是低频扩展且需要数组过滤语义; modules: migration、metadata; target: `rag_project_doc_sources`; recommendation: 暂存 metadata，不进入本切片 filterSchema; decision: approved_with_defaults.
