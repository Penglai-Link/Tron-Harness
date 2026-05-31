# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, POST /api/v1/rag/search, Internal Rust Contract, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, +1 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, rag_profiles seed, Search Read Models, Optional Indexes, Relationships And Cardinality, Migration Plan, +4 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Decomposition Summary, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, +13 more | `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#检索`, `Feature: RAG 后端底座 - 检索编排与 Context Pack`, `Page: Backend API - RAG Search`, `Interaction: RAG Search - 提交检索请求`, `Rule: RAG 后端底座 - 检索 Scope`, `Rule: RAG 后端底座 - Version Filter`, `Rule: RAG 后端底座 - CandidateHit 统一结构`, `Rule: RAG 后端底座 - Embedding 降级`, `Rule: RAG 后端底座 - Filter Schema`, `Exception: RAG 后端底座 - 缺少版本过滤`, `Exception: RAG 后端底座 - 向量 route 不可用`, `Exception: RAG 后端底座 - Hydrate 后 Source 不可见`, +1 more | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Downstream Impact, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Changed Paths, Blockers | none | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 |
