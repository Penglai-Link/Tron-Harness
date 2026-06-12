# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, POST /api/v1/rag/ingest, Internal Rust Contract, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, +1 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, rag_profiles seed, rag_memory_sources write fields, rag_project_doc_sources write fields, TextUnit tables, rag_diagnostics warnings, +6 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +14 more | `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#第一阶段目标`, `Feature: RAG 后端底座 - 同步写入 DAG`, `Page: Backend API - RAG Ingest`, `Rule: RAG 后端底座 - 写入 Scope`, `Rule: RAG 后端底座 - Source 版本幂等`, `Rule: RAG 后端底座 - Embedding 降级`, `Rule: RAG 后端底座 - Filter Schema`, `Exception: RAG 后端底座 - DAG 配置非法`, `Exception: RAG 后端底座 - Embedding 失败`, `Exception: RAG 后端底座 - 重复写入同版本`, `Acceptance: RAG 后端底座 - 写入链路`, `Acceptance: RAG 后端底座 - Embedding 与 Rerank`, `Acceptance: RAG 后端底座 - Scope 与访问边界` | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Downstream Impact, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Changed Paths, Blockers | none | AC-RID-01, AC-RID-02, AC-RID-03, AC-RID-04, AC-RID-05, AC-RID-06, AC-RID-07 |
