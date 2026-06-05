# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, GET /api/v1/tasks/options - Task options, GET /api/v1/tasks/kanban - Task kanban, GET /api/v1/tasks - Task list, POST /api/v1/tasks - Create task, GET /api/v1/tasks/{taskCode} - Task detail, PATCH /api/v1/tasks/{taskCode} - Update task, +8 more | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, tasks, task_user_assignments, task_agent_links, task_app_links, Relationships And Cardinality, +5 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +5 more | none | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL, CMD-DOCS-SPEC, CMD-FE-CHECK, CMD-TASK-BROWSER | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-08 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-TASK-08 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-TASK-07, AC-TASK-08, AC-TASK-09 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | AC-TASK-09 |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| reports/agents/AC-TASK-07-lead-task-management-smoke.md | evidence-ledger | evidence rows | Interaction: AC-TASK-07, Clicks, Inputs, Network Assertions, Visible Feedback Assertions, Artifacts, Status: pass | none | AC-TASK-07 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-08 |
| reports/database-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-TASK-08 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-TASK-07, AC-TASK-08, AC-TASK-09 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-TASK-01, AC-TASK-02, AC-TASK-03, AC-TASK-04, AC-TASK-05, AC-TASK-06, AC-TASK-07, AC-TASK-08, +1 more |
