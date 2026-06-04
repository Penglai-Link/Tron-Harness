# Feature Spec: RAG Skill Graph Corpus

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_by_user_default

本切片把已讨论确认的方案落成 RAG 内部能力：保留现有 `skill` SQLite 语料库不变，新增独立 `skill-graph` 语料库，让一个 skill 作为一个图节点写入和检索，并在可选配置下写入及检索与 skill 相关的图节点和边。

关键默认值：图谱检索关系扩展在 rerank 和最终 limit 之后执行；扩展结果只附加到候选命中的 metadata，不改变顶层 HTTP schema；所有测试使用 fake embedding/rerank runtime，不访问公司模型网络。

## Feature Identity And Context

- feature_slug: rag-skill-graph-corpus
- feature_name: RAG Skill Graph Corpus
- status: approved_with_defaults
- owner: langtaoxin
- source_request: 用户要求基于已确认方案，用 codex-superpower 完成 RAG skill 数据新增 graph 存储实现、存储策略、检索策略、边界验证和证据报告。
- main_agent: current-thread lead agent
- dependency_features: `graph-scope-and-isolation-kernel`, `embedded-grafeo-adapter-foundation`, `rag-search-context-pack`, `skill-rag-corpus`
- dependency_contracts: `tron-backend/src/domain/rag/model.rs`, `tron-backend/src/application/rag/ingest.rs`, `tron-backend/src/application/rag/search.rs`, `tron-backend/src/application/graph/service.rs`, `tron-backend/src/adapters/outbound/persistence/grafeo_graph_store.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`

## Requirement

- user_story: 作为 RAG skill 检索调用方，我需要在不破坏现有 `skill`、memory 和 project-doc 写入检索的前提下，新增一个能把 skill 及其关系数据存入 graph 的 `skill-graph` 语料库，并在检索最终结果中带回相关图数据。
- intent: 让 RAG 可以选择 graph 作为 skill 数据底层存储介质，同时保留 profile-driven 的写入、检索、debug 和 retry 执行模型。
- outcome: `skill-graph` 可独立写入、检索和关系扩展；旧 `skill` 仍走 SQLite；其他 RAG corpus 不受影响。

## Product Source Slices

- brainstorming_source: current conversation approved design notes for RAG skill graph storage and retrieval order
- prd_feature_headings: not applicable; user-approved technical evolution slice
- prd_page_headings: not applicable
- prd_rule_headings: existing RAG ingest/search/profile rules plus graph scope/storage rules
- prd_exception_headings: embedding/rerank external network unavailable in local testing
- prd_acceptance_headings: current conversation final scheme and boundaries

## Scope Boundary

### Assumptions

- 当前 RAG ingest 和 search profile 机制已经可通过 seeded corpus/profile 控制数据流。
- 当前 graph storage engine 已支持 scoped node、edge、fulltext、semantic projection 和 traversal。
- `skill-graph` 第一版只服务 RAG skill 数据，不替换 Skill Center 或普通 skill 模块存储。
- 一个 skill 对应一个 `RagSkill` 图节点；不再拆 Source 节点和 TextUnit 节点。
- 关系写入和关系扩展均由 profile policy 控制，不接受 raw graph query。

### In Scope

- 新增 `GRAPH_SINGLE_NODE` RAG storage layout。
- 新增 `skill-graph` corpus、ingest profile 和 retrieval profile seed。
- 新增 graph-backed RAG corpus store/router，用于写入 `RagSkill` 节点并复用 RAG 阶段生成的 embedding。
- 支持 payload 中的 typed related graph 节点和边，在 policy 允许时随 skill 一起写入。
- 新增 graph fulltext/vector route，实现 `skill-graph` 检索并进入现有 rank、fusion、dedup、rerank 和 context pack 流程。
- 在 rerank 和最终 limit 后，对最终 `skill-graph` 命中执行受控关系扩展，并把结果放入 hit metadata。
- 修复 RAG 内部 dedup key，使不同 corpus、sourceVersion 和 scope 的同名 target 不互相覆盖。
- debug source 和 retry index 按 storage layout 路由，graph layout 返回图节点状态或明确的阶段不支持错误。

### Out Of Scope

- 不删除、不迁移、不默认替换现有 `skill` 语料库。
- 不改变 memory-card、project-doc 的写入或检索策略。
- 不改变 Skill Center、skill registry 或 workspace binding 的业务存储。
- 不新增公开 graph 查询 HTTP API。
- 不改变 RAG search 顶层响应 schema。
- 不实现 rerank 前的 context expansion。
- 不允许客户端提交 raw graph query、raw traversal DSL 或未白名单关系。

### Constraints

- 必须优先使用 Makefile 做 build/test/cleanup。
- embedding 和 rerank 验证必须使用 fake/stub runtime，不能依赖公司模型网络。
- graph business properties 不能使用 graph engine 保留属性名。
- `RagSkill` 节点 code 必须稳定、可重复、符合 graph code 规范。
- Graph scope 仍按 RAG 的 USER、AGENT、SPACE direct scopes 执行，graph store 的 AND scope 语义通过多 scope 查询合并实现。

## Feature Model

### Corpus Coexistence

`skill` 与 `skill-graph` 是两个独立 corpus。`skill` 保持当前 SQLite embedded-source 写入检索；`skill-graph` 通过 storage binding 指向 graph namespace、node label 和 code strategy。调用方只有把 profile 切到 `skill-graph` 才进入图谱存储。

### Single Node Skill Layout

每个 skill 写成一个 `RagSkill` 节点。节点 properties 同时保留 source 与 text unit 兼容字段，例如 corpus id、source code、source version、source type、scope、target code、skill name、title、text、search text、status、token count、metadata 和 embedding status。正文用于 fulltext，已有 RAG embedding 用于 semantic payload。

### Related Graph Write Policy

写入 payload 可携带 typed related graph bundle。Profile policy 定义允许的 node kinds、relationships、数量上限和失败策略。`$skill` 表示当前 skill 节点。第一版相关节点可统一写为 `RagSkillRelatedNode`，通过 `relatedKind` 区分工具、配置、变量、agent 或 tag。

### Search And Expansion

`skill-graph` route 返回普通 `RagSearchRow`，继续走现有 route rank、fusion、dedup、topic policy、rerank。最终命中截断后，只对来自 graph layout 的 final hits 做关系扩展，并把 related nodes/edges 放入 CandidateHit metadata。

### Debug And Retry

debug 按 corpus storage layout 路由。graph layout 从 `RagSkill` 节点和 diagnostics 读取状态。retry 对 fulltext 和 vector stage 使用 graph node 的 text/searchText 与 semantic payload 更新能力；暂不支持的 stage 返回 typed unsupported error。

## Page And Module Surface

UI: Not applicable.

### Backend Modules

- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest_payload.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_ranking.rs`
- `tron-backend/src/application/rag/diagnostics.rs`
- `tron-backend/src/application/rag/retry.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/grafeo_graph_store.rs`
- `tron-backend/migrations`
- `tron-backend/tests`

## Repository Findings

- `RagStorageLayout` 当前只有 `SPLIT` 和 `EMBEDDED_SOURCE`，需要扩展 layout enum 和 storage binding 读取。
- RAG ingest 当前只调用 repository `persist_ingest`，适合在 repository 层或复合 store 层按 layout dispatch。
- RAG search 当前 repository 只提供 `search_route`，新增 graph route 可以在同一 repository trait 后面实现而不改 HTTP schema。
- GraphStore 已暴露 prepared node/edge write 和 prepared search query，可复用 RAG embedding，避免二次调用模型。
- Graph scope storage filters 是 namespace、workspace、app、user 的 AND 语义，而 RAG search scope 是 USER、AGENT、SPACE OR 语义，需要多 scope 查询合并。
- 当前 `fuse_and_dedup` 已有 canonical key 字段，但需要确认 key 是否包含 corpus、sourceVersion 和 scope，避免 `skill` 与 `skill-graph` 冲突。
- debug 和 retry 现有 SQLite 实现需要按 storage layout 分流，不能让 graph corpus 落入 SQLite table plan。

## Open Decisions

- Graph-backed RAG store is reusable for future corpora; signal: additional graph-backed corpus may reuse storage layout and route code; modules: RAG repository and graph adapter; target: feature-local first, extract behind trait when second corpus appears; recommendation: defer; decision: approved default.
