# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Auth And Permissions, Endpoints, GET /api/v1/rag/debug/profile, Query, Success Data, GET /api/v1/rag/debug/source, POST /api/v1/rag/retry/index, +10 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Proposed Schema, Existing Schema Impact, Diagnostics Records, INDEXING_WARNING, SEARCH_TRACE, Retry Updates, Fulltext Retry, +10 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Decomposition Summary, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, +15 more | `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#已确认范围`, `Feature: RAG 后端底座 - 诊断、重试、Trace 与事件`, `Page: Backend API - RAG Debug`, `Page: Backend API - RAG Indexing Retry`, `Interaction: RAG Debug - 查看诊断数据`, `Interaction: RAG Indexing Retry - 重试单 Source 索引`, `Rule: RAG 后端底座 - Warnings 持久化`, `Rule: RAG 后端底座 - Trace 隐私边界`, `Rule: RAG 后端底座 - Retry 不重跑解析`, `Exception: RAG 后端底座 - Retry Source 不存在`, `Exception: RAG 后端底座 - Trace TTL 到期`, `Exception: RAG 后端底座 - Event payload 越界`, +1 more | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Downstream Impact, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 |
