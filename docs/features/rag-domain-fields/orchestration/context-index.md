# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Internal Rust Contract, Memory Filter Example, Project Doc Filter Example, Auth And Permissions, Error Contract, Frontend Backend Boundary, +2 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, rag_memory_sources, rag_project_doc_sources, rag_corpora seed, Relationships And Cardinality, Migration Plan, +4 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +12 more | `docs/brainstorm/rag-backend-foundation/2026-05-30-16-34-57.md#已确认范围`, `Feature: RAG 后端底座 - Memory 与 Project Doc 领域字段`, `Page: Backend API - RAG Ingest`, `Page: Backend API - RAG Search`, `Rule: RAG 后端底座 - Memory 治理字段只存储`, `Rule: RAG 后端底座 - Metadata 使用边界`, `Rule: RAG 后端底座 - Filter Schema`, `Exception: RAG 后端底座 - 未声明过滤字段`, `Exception: RAG 后端底座 - Metadata 字段误用`, `Acceptance: RAG 后端底座 - 写入链路`, `Acceptance: RAG 后端底座 - 检索链路`, `Acceptance: RAG 后端底座 - Scope 与访问边界` | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-SPEC | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Commands, Evidence Rows, Changed Paths, AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, +4 more | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 |
