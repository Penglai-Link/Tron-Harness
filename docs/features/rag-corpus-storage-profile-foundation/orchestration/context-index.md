# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RAG-01, AC-RAG-02, AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-08, +1 more |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Internal Rust Contract, DTO Shape, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, +1 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Table Count, Proposed Schema, rag_corpora, rag_profiles, rag_memory_sources, rag_project_doc_sources, +13 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +14 more | `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#已确认范围`, `Module: RAG 后端底座`, `Feature: RAG 后端底座 - Corpus 存储与 Profile 基础`, `Page: Backend API - RAG Debug`, `Rule: RAG 后端底座 - Corpus 边界`, `Rule: RAG 后端底座 - Profile 与 Registry 边界`, `Rule: RAG 后端底座 - Filter Schema`, `Exception: RAG 后端底座 - Profile 缺失或策略未注册`, `Exception: RAG 后端底座 - 非法过滤字段`, `Acceptance: RAG 后端底座 - 第一阶段范围`, `Acceptance: RAG 后端底座 - Scope 与访问边界` | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-SPEC | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RAG-01, AC-RAG-02, AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-08, +1 more |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-09 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RAG-01, AC-RAG-02, AC-RAG-08 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RAG-01, AC-RAG-02, AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-08, +1 more |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-RAG-01, AC-RAG-02, AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-08, +1 more |
