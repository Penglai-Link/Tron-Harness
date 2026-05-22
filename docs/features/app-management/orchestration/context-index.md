# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | ac_refs |
| --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, GET /api/v1/apps/workspaces/{workspaceCode} - Workspace App List, POST /api/v1/apps/workspaces/{workspaceCode} - Create App, GET /api/v1/apps/workspaces/{workspaceCode}/options - Active App Options, GET /api/v1/apps/{appCode} - App Detail, PATCH /api/v1/apps/{appCode} - Update App, POST /api/v1/apps/{appCode}/archive - Archive App, +8 more | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Table: apps, Table: app_tags, Reused Table: task_app_links, Relationships And Cardinality, Migration Plan, +4 more | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, +4 more | none |
| spec/test-plan.md | proof-plan | CMD-BE-TEST, CMD-DOC-VALIDATE, CMD-FE-CHECK, CMD-PW-APP, CMD-PW-TASK | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-APP-04, AC-APP-05, AC-APP-06 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-APP-04, AC-APP-05, AC-APP-06 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-06 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, #1, #2, #3, Downstream Artifact Migration, Residual Risks | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, #1, #2, #3, Conflict Log, Validation After Integration, Blockers | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-APP-04, AC-APP-05, AC-APP-06 |
| reports/database-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-APP-04, AC-APP-05, AC-APP-06 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-06 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-APP-01, AC-APP-02, AC-APP-03, AC-APP-04, AC-APP-05, AC-APP-06 |
