# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | @ref:spec/feature-spec.md#Product Source Slices | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | @ref:spec/feature-spec.md#Page And Module Surface | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, Validation Commands | @ref:spec/feature-spec.md#Feature Model | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Product Source Slices, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, Feature Model, Page And Module Surface, Repository Findings, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | none |
| spec/test-plan.md | proof-plan | CMD-* | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | @ref:spec/acceptance.md#Acceptance Index | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | AC-AGENT-04, AC-AGENT-03, AC-AGENT-01 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | @ref:spec/feature-spec.md#Feature Identity And Context | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | @ref:orchestration/worktree-plan.md#Dispatch Decisions | none |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | none |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-04, AC-AGENT-03, AC-AGENT-01 |
| reports/database-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Changed Paths, Blockers | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Findings, Changed Paths, Commands, Residual Risks, Blockers | @ref:spec/test-plan.md#Proof Rows | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | @ref:orchestration/worktree-plan.md#Dispatch Decisions | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03, AC-AGENT-04 |
