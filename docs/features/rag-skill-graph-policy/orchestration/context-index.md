# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-RSGP-01, AC-RSGP-02, AC-RSGP-03, AC-RSGP-04, AC-RSGP-05, AC-RSGP-06, AC-RSGP-07 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Existing RAG Ingest API, Existing RAG Search API, Auth And Permissions, Error Contract, Frontend Backend Boundary, Database And Transaction Boundary, +1 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Relationships And Cardinality, Migration Plan, Query And Performance Plan, Data Governance, Test Data And Fixtures, +1 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, Constraints, In Scope, +9 more | current conversation approved RAG skill graph policy decisions, not applicable; technical feature evolution, graph write policy optional whitelist, field mapping business fields, scalar edge properties, tests must not use company embedding or rerank network, approved graph policy acceptance rows | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-BE-TEST, CMD-DOCS-FINAL, CMD-RSGP-FOCUSED, CMD-RSGP-STATIC | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-RSGP-01, AC-RSGP-02, AC-RSGP-03, AC-RSGP-04, AC-RSGP-05, AC-RSGP-06, AC-RSGP-07 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-RSGP-01, AC-RSGP-02, AC-RSGP-03, AC-RSGP-04, AC-RSGP-05, AC-RSGP-06, AC-RSGP-07 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, Downstream Artifact Migration, Downstream Impact, Residual Risks | none | none |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, Conflict Log, Validation After Integration, Blockers | none | none |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-RSGP-01, AC-RSGP-02, AC-RSGP-03, AC-RSGP-04, AC-RSGP-05, AC-RSGP-06, AC-RSGP-07 |
| reports/backend-result.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Changed Paths, Blockers | none | AC-RSGP-01, AC-RSGP-02, AC-RSGP-03, AC-RSGP-04, AC-RSGP-05, AC-RSGP-06, AC-RSGP-07 |
