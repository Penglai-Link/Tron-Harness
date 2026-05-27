# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Request And Response DTOs, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations, +1 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, +2 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +8 more | `docs/brainstorm/agent-framework/2026-05-26-22-18-36.md`, `Feature Inventory` row `Agent Invocation API`, `Rule: Backend Agent Framework - Run Isolation`, `Rule: Backend Agent Framework - Context Snapshot`, `Rule: Backend Agent Framework - Frozen Model Config`, `Rule: Backend Agent Framework - Event Payload Boundary`, `Exception: Backend Agent Framework - Invalid Run Configuration`, `Exception: Backend Agent Framework - Model Failure`, `Exception: Backend Agent Framework - Max Step Exhaustion`, `Exception: Backend Agent Framework - Event Publish Failure` | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-BROWSER-SMOKE, CMD-DOCS-SPEC, CMD-FE-CHECK | Test Plan Decision, Proof Rows, Command Index, Evidence Rules, Open Decisions | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AIA-05 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04 |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AIA-05 |
| reports/review-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
| reports/worktree-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AIA-01, AC-AIA-02, AC-AIA-03, AC-AIA-04, AC-AIA-05 |
