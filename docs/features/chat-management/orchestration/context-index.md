# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-04, AC-CM-05, AC-CM-06, AC-CM-07 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, GET /api/v1/chats/session-groups - List Chat Session Groups, POST /api/v1/chats/sessions - Create Chat Session, GET /api/v1/chats/sessions/{sessionCode}/messages - List Chat Messages, POST /api/v1/chats/sessions/{sessionCode}/messages - Send Chat Message, Auth And Permissions, Error Contract, +3 more | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-05 |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, chat_sessions, chat_messages, Relationships And Cardinality, Migration Plan, Query And Performance Plan, +3 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Requirement, Product Source Slices, Scope Boundary, Assumptions, In Scope, Out Of Scope, +5 more | implementation request, `Chat 管理页设计总结`, approved-with-defaults from implementation request, `页面布局`, `左栏 Session Sidebar`, `中栏 对话界面`, `右栏 Right Context Sidebar`, `概念边界`, `后端边界`, context window unavailable fallback, unassigned sessions, recommended landing order in request | none |
| spec/test-plan.md | proof-plan | CMD-BE-TEST, CMD-DOCS-SPEC, CMD-FE-CHECK | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-04, AC-CM-05, AC-CM-06, AC-CM-07 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-CM-01, AC-CM-02, AC-CM-05 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-CM-03, AC-CM-04, AC-CM-06, AC-CM-07 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-04, AC-CM-05, AC-CM-06, AC-CM-07 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | Summary, Change History, #2, #1, Downstream Artifact Migration, Residual Risks | none | AC-CM-05, AC-CM-06, AC-CM-07 |
| orchestration/integration-log.md | orchestration | supporting facts | Summary, Integration Events, #1, Conflict Log, Validation After Integration, Blockers | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-04, AC-CM-05, AC-CM-06, AC-CM-07 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Dispatch Decisions, Agent Worktrees, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-04, AC-CM-05, AC-CM-06, AC-CM-07 |
| reports/test-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Changed Paths, Commands, Blockers | none | AC-CM-01, AC-CM-02, AC-CM-03, AC-CM-04, AC-CM-05, AC-CM-06, AC-CM-07 |
