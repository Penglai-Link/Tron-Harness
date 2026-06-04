# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Decomposition Summary, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Feature Model, API Surface, Page And Module Surface, Repository Findings, Open Decisions | current thread approved structured-filter scheme | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, Validation Commands | none | none |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, POST /api/v1/rag/search, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | none | none |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
| spec/test-plan.md | proof-plan | CMD-BE-RAG-SEARCH, CMD-BE-RAG-API, CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Downstream Impact, Residual Risks | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, Open Decisions | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Changed Paths, Blockers | none | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 |
