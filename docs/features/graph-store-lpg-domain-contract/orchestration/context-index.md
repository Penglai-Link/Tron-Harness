# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-LPG-01, AC-LPG-02, AC-LPG-03, AC-LPG-04, AC-LPG-05, AC-LPG-06, AC-LPG-07 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Internal Rust Contract, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Phase 1: Typed Port Contract, Future Grafeo Adapter Phase, Relationships And Cardinality, Migration Plan, Query And Performance Plan, +3 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +9 more | none | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-FMT, CMD-BE-LPG-GUARD, CMD-BE-LPG-UNIT | Test Plan Decision, Proof Rows, Command Index, Evidence Rules, Regression Scope | none | AC-LPG-01, AC-LPG-02, AC-LPG-03, AC-LPG-04, AC-LPG-05, AC-LPG-06, AC-LPG-07 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-LPG-01, AC-LPG-02, AC-LPG-03, AC-LPG-04, AC-LPG-05, AC-LPG-06, AC-LPG-07 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | none |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-LPG-01, AC-LPG-02, AC-LPG-03, AC-LPG-04, AC-LPG-05, AC-LPG-06, AC-LPG-07 |
