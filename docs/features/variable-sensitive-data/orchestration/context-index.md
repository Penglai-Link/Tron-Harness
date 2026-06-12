# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-VSD-01, AC-VSD-02, AC-VSD-03, AC-VSD-04, AC-VSD-05, AC-VSD-06, AC-VSD-07, AC-VSD-08, +2 more |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, +1 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +5 more | none | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-VARIABLE-TEST, CMD-DOCS-FINAL, CMD-DOCS-TASKS, CMD-DOCS-WORKTREES, CMD-FE-CHECK, CMD-VSD-BROWSER | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-VSD-01, AC-VSD-02, AC-VSD-03, AC-VSD-04, AC-VSD-05, AC-VSD-06, AC-VSD-07, AC-VSD-08, +2 more |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-VSD-03, AC-VSD-04, AC-VSD-05, AC-VSD-06, AC-VSD-10 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-VSD-01, AC-VSD-02 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-VSD-07, AC-VSD-08, AC-VSD-09 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-VSD-01, AC-VSD-02, AC-VSD-03, AC-VSD-04, AC-VSD-05, AC-VSD-06, AC-VSD-07, AC-VSD-08, +2 more |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-VSD-09, AC-VSD-10 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Backend Merge, Frontend Merge, Conflict Log, Validation After Integration, Blockers | none | none |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, WT-01, WT-02, Branch And Path Rules, +4 more | none | AC-VSD-01, AC-VSD-02, AC-VSD-03, AC-VSD-04, AC-VSD-05, AC-VSD-06, AC-VSD-07, AC-VSD-08, +1 more |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-VSD-01, AC-VSD-02, AC-VSD-03, AC-VSD-04, AC-VSD-05, AC-VSD-06, AC-VSD-10 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-VSD-07, AC-VSD-08, AC-VSD-09 |
