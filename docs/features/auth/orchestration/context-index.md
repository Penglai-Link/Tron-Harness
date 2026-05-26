# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, +1 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Product Source Slices, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, +5 more | docs/prd.typ#Feature: Console Access and Identity; docs/brainstorm/unassigned-space-authorization/2026-05-26-15-29-24.md#Decision | none |
| spec/test-plan.md | proof-plan | CMD-BE-BOOTSTRAP-ADMIN, CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL, CMD-FE-AUTH-BROWSER, CMD-FE-CHECK | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-05, AC-AUTH-07, AC-AUTH-08, AC-AUTH-10 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AUTH-06, AC-AUTH-07, AC-AUTH-10 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AUTH-03, AC-AUTH-04, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | AC-AUTH-09, AC-AUTH-10 |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, #1, Conflict Log, Validation After Integration, Blockers | none | AC-AUTH-01 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, +1 more | none | AC-AUTH-09, AC-AUTH-10 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| reports/database-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AUTH-01, AC-AUTH-02, AC-AUTH-03, AC-AUTH-04, AC-AUTH-05, AC-AUTH-06, AC-AUTH-07, AC-AUTH-08, AC-AUTH-09, AC-AUTH-10 |
