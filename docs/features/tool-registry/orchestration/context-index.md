# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | @ref:spec/feature-spec.md#Product Source Slices | AC-TOOL-01, AC-TOOL-02, AC-TOOL-03, AC-TOOL-04, AC-TOOL-05 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Auth And Permissions, Existing Agent Management Compatibility, Runtime Discovery Contract, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, Mock Or Adapter Expectations | @ref:spec/feature-spec.md#Page And Module Surface | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Validation Commands | @ref:spec/feature-spec.md#Feature Model | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Product Source Slices, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, Constraints, Feature Model, Page And Module Surface, Repository Findings, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | none |
| spec/test-plan.md | proof-plan | CMD-* | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | @ref:spec/acceptance.md#Acceptance Index | AC-TOOL-01, AC-TOOL-02, AC-TOOL-03, AC-TOOL-04, AC-TOOL-05 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | AC-TOOL-01, AC-TOOL-02, AC-TOOL-03 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | AC-TOOL-04, AC-TOOL-05 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | AC-TOOL-02 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/test-plan.md#Proof Rows | AC-TOOL-01, AC-TOOL-02, AC-TOOL-03, AC-TOOL-04, AC-TOOL-05 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | @ref:spec/test-plan.md#Proof Rows | AC-TOOL-01, AC-TOOL-02, AC-TOOL-03, AC-TOOL-04, AC-TOOL-05 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, Cleanup Plan, Open Decisions | @ref:spec/acceptance.md#Acceptance Index | none |
