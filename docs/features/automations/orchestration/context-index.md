# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and acceptance rows.

| file | kind | owns | headings | ac_refs |
| --- | --- | --- | --- | --- |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, Feature Model, Page And Module Surface, Repository Findings, Open Decisions | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, Validation Commands | none |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | none |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | AC-AUTO-01, AC-AUTO-02, AC-AUTO-03, AC-AUTO-04, AC-AUTO-05, AC-AUTO-06, AC-AUTO-07, AC-AUTO-08 |
| spec/test-plan.md | proof-plan | CMD-AUTO-BROWSER, CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL, CMD-DOCS-SPEC, CMD-FE-CHECK | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | AC-AUTO-01, AC-AUTO-02, AC-AUTO-03, AC-AUTO-04, AC-AUTO-05, AC-AUTO-06, AC-AUTO-07, AC-AUTO-08 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-AUTO-03, AC-AUTO-05 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-AUTO-01, AC-AUTO-02, AC-AUTO-03, AC-AUTO-04, AC-AUTO-05, AC-AUTO-06 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-AUTO-07, AC-AUTO-08 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-AUTO-01, AC-AUTO-02, AC-AUTO-03, AC-AUTO-04, AC-AUTO-05, AC-AUTO-06, AC-AUTO-07, AC-AUTO-08 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-AUTO-01, AC-AUTO-02, AC-AUTO-03, AC-AUTO-04, AC-AUTO-05, AC-AUTO-06, AC-AUTO-07, AC-AUTO-08 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, Open Decisions | AC-AUTO-01, AC-AUTO-02, AC-AUTO-03, AC-AUTO-04, AC-AUTO-05, AC-AUTO-06, AC-AUTO-07, AC-AUTO-08 |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none |
