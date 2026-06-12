# Feature Spec: RAG Skill Graph Policy

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user

本切片演进已存在的 `skill-graph` corpus。用户已确认：`graphWritePolicy.allowedNodeKinds` 和 `allowedRelationships` 只有存在且非空时才校验；业务字段从 corpus field mapping 读取；RAG 技术字段保持固定；`relatedGraph.edges[].properties` 支持标量属性；检索侧关系过滤保持请求驱动。

## Feature Identity And Context

- feature_slug: rag-skill-graph-policy
- feature_name: RAG Skill Graph Policy
- status: approved_with_defaults
- owner: langtaoxin
- source_request: 用户要求使用 codex-superpower、AC 拆分、git worktree 隔离、可并行 agent、TDD 验证完成 RAG skill graph 写入策略配置化。
- dependency_features: `rag-skill-graph-corpus`, `rag-domain-fields`, `graph-scope-and-isolation-kernel`
- dependency_contracts: `tron-backend/src/application/rag/ingest.rs`, `tron-backend/src/application/rag/ingest_profile.rs`, `tron-backend/src/application/rag/search.rs`, `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/ingest.rs`, `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/model.rs`, `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/search.rs`

## Requirement

- user_story: 作为 RAG skill graph 调用方，我希望相关节点类型、关系类型和业务字段不写死在代码里，而是由配置和请求驱动，这样后续 agent 能传入更多业务关系。
- intent: 降低 `skill-graph` 业务耦合，同时保留 RAG 运行字段和现有检索链路稳定性。
- outcome: 默认 seed 不限制 kind/relationship；显式配置白名单时才拒绝；动态 field mapping 字段能落图和返回；edge properties 能写入并随 graph expansion 返回。

## Product Source Slices

- brainstorming_source: current conversation approved RAG skill graph policy decisions
- prd_feature_headings: not applicable; technical feature evolution
- prd_page_headings: not applicable
- prd_rule_headings: graph write policy optional whitelist, field mapping business fields, scalar edge properties
- prd_exception_headings: tests must not use company embedding or rerank network
- prd_acceptance_headings: approved graph policy acceptance rows

## Scope Boundary

### Assumptions

- `skill-graph` corpus and base graph storage path already exist.
- Existing `skill`, memory, and project-doc corpora remain configured independently.
- Graph adapter internal labels are storage implementation details, not RAG business policy.
- Existing request-time graph expansion relationship filter is already present and should remain.

### Constraints

- Use Makefile first for backend build/test verification.
- No test may require external embedding or rerank provider access.
- Do not modify non-RAG modules except existing app wiring already present in the base worktree.
- Preserve graph scope semantics for `USER`, `AGENT`, and `SPACE`.

### In Scope

- Compile and pass `graphWritePolicy` to graph-backed persist.
- Parse optional `allowedNodeKinds` and `allowedRelationships` from profile config.
- Remove production default business kind and relationship sets.
- Write and return main-node business fields from corpus field mapping.
- Support scalar edge properties in write and expansion response.
- Keep graph expansion relationship filtering request-driven.

### Out Of Scope

- No new public graph query API.
- No label allowlist or kind-to-label derivation.
- No broad filter operator/type expansion beyond mapping logical fields to graph properties.
- No conversion of RAG technical fields into configuration.

## Feature Model

### Optional Graph Write Whitelists

`graphWritePolicy.allowedNodeKinds` and `allowedRelationships` are optional arrays. Missing or empty arrays mean no business whitelist. Present non-empty arrays reject request values outside the configured set. Values still need to be valid uppercase graph business codes.

### Field Mapped Business Properties

Graph skill main-node properties are split into fixed RAG technical fields and field-mapped business fields. Domain fields are accepted only when corpus field mapping declares their graph property name, then written as graph scalar properties and returned through search result fields.

### Edge Properties

`relatedGraph.edges[].properties` accepts scalar values only: string, integer, decimal, and boolean. Object, array, null, empty property keys, and reserved edge keys are invalid.

### Retrieval Boundary

Retrieval keeps the existing order: route search, rank, fuse/dedup, optional rerank, final limit, graph expansion, context assembly. Relationship filter values come from the search request; profile keeps direction, max depth, max related nodes, and failure policy.

## Page And Module Surface

UI: Not applicable.

Backend modules:

- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest_profile.rs`
- `tron-backend/src/application/rag/ingest/executor.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/ingest.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/model.rs`
- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/search.rs`
- `tron-backend/migrations/20260604_0031_create_skill_graph_rag_corpus.sql`
- `tron-backend/tests/rag_skill_graph_contracts.rs`

## Repository Findings

- `graphWritePolicy` existed in the seed migration but production code used `corpus_policy_defaults`.
- Main graph node business fields were hardcoded to `skillName` and `skillDescription`.
- Edge payload properties were ignored and expansion returned no edge property object.
- Search graph filters used request field names directly; mapping through corpus field mapping is safer for business aliases.
- Graph scope `AGENT` is a RAG technical scope and remains fixed.

## Open Decisions

- Label allowlist remains out of scope because the user explicitly requires related node label from request.
