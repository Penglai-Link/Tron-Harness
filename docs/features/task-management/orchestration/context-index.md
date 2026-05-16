# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | ac_refs |
| --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06 |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, Validation Commands | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, Feature Model, Page And Module Surface, Repository Findings, Open Decisions | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL, CMD-DOCS-SPEC, CMD-FE-CHECK, CMD-TASK-BROWSER | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-TASK-02, AC-TASK-06 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-TASK-07, AC-TASK-08 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, Open Decisions | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06 |
| reports/database-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-TASK-02, AC-TASK-06 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-TASK-07, AC-TASK-08 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08 |
