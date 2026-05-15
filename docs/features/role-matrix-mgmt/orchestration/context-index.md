# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | ac_refs |
| --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | AC-RM-01, AC-RM-02, AC-RM-03, AC-RM-04, AC-RM-05, AC-RM-06, AC-RM-07 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Read Endpoints, Write Endpoint, Frontend Working-Copy Contract, Auth And Permissions, Error Contract, Frontend Backend Boundary, +2 more | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, +1 more | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, +4 more | none |
| spec/test-plan.md | proof-plan | CMD-BE-TEST, CMD-FE-CHECK, CMD-RM-BROWSER | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | AC-RM-01, AC-RM-02, AC-RM-03, AC-RM-04, AC-RM-05, AC-RM-06, AC-RM-07 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | AC-RM-01, AC-RM-02, AC-RM-03, AC-RM-04, AC-RM-05, AC-RM-06, AC-RM-07 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, #1, Conflict Log, Validation After Integration, Blockers | AC-RM-01, AC-RM-02, AC-RM-03, AC-RM-04, AC-RM-05, AC-RM-06, AC-RM-07 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, +1 more | none |
| reports/frontend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | AC-RM-01, AC-RM-02, AC-RM-03, AC-RM-04, AC-RM-05, AC-RM-06, AC-RM-07 |
