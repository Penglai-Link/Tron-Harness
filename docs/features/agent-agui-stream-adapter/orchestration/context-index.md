# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations, Open Decisions | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, +2 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +7 more | `Feature Inventory` row `Agent AG-UI Stream Adapter`, event filtering and protocol conversion from the feature inventory, disconnected consumer and stale stream from the feature inventory | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-BROWSER-SMOKE, CMD-DOCS-SPEC, CMD-FE-CHECK | Test Plan Decision, Proof Rows, Command Index, Evidence Rules, Open Decisions | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AAS-01, AC-AAS-02, AC-AAS-03 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AAS-04 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-AAS-01, AC-AAS-02, AC-AAS-03, AC-AAS-04 |
