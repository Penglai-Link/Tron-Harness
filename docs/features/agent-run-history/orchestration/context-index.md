# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations, Open Decisions | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, `agent_runs`, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, +3 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +7 more | `Feature Inventory` row `Agent Run History`, transcript retention, masking, and replay access from the feature inventory, oversized transcript and missing run from the feature inventory | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-SPEC | Test Plan Decision, Proof Rows, Command Index, Evidence Rules, Open Decisions | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-ARH-01, AC-ARH-02, AC-ARH-03, AC-ARH-04, AC-ARH-05 |
