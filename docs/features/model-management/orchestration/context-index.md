# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | ac_refs |
| --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, Validation Commands | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, Feature Model, Page And Module Surface, Repository Findings, Open Decisions | none |
| spec/test-plan.md | proof-plan | CMD-* | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-MODEL-04 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, Open Decisions | none |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| reports/database-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-MODEL-04 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Changed Paths, Blockers | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Findings, Changed Paths, Commands, Residual Risks, Blockers | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-MODEL-01, AC-MODEL-02, AC-MODEL-03, AC-MODEL-04 |
