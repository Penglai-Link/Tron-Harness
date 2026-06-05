# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | Acceptance Index, Open Decisions | none | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 |
| spec/api-contract.md | spec-support | supporting facts | API Overview, Endpoints, Platform Gateway, Platform MCP Server Registry, Workspace Effective MCP Servers, Internal-Agent MCP Policy, MCP Transport Endpoint, Auth And Permissions, +5 more | none | none |
| spec/database-design.md | spec-support | supporting facts | Database Decision, Existing Schema Impact, Proposed Schema, Table: platform_mcp_gateway_configs, Table: platform_mcp_servers, Table: workspace_mcp_server_policies, Changed Table: workspace_internal_agent_policies, Table: workspace_internal_agent_mcp_server_assignments, +6 more | none | none |
| spec/feature-spec.md | feature-model | feature context | Initial Agent Draft, Feature Identity And Context, Product Source Slices, Requirement, Scope Boundary, Assumptions, In Scope, Out Of Scope, +5 more | `docs/brainstorm/mcp-management/2026-06-05-23-07-39.md` | none |
| spec/test-plan.md | proof-plan | CMD-BE-CHECK, CMD-DOCS-FINAL, CMD-FE-CHECK, CMD-MCP-BE-TEST, CMD-MCP-BROWSER | Test Plan Decision, Proof Rows, Command Index, Evidence Rules | none | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 |
| tasks/backend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04 |
| tasks/database-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-MCP-01, AC-MCP-03 |
| tasks/frontend-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-MCP-02, AC-MCP-03, AC-MCP-05 |
| tasks/review-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 |
| tasks/test-task.md | task-plan | execution rows | Execution Contract, TDD Contract, Task Rows, Worktree Notes, Integration Notes, Open Decisions | none | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 |
| orchestration/worktree-plan.md | orchestration | supporting facts | Main Agent, Repository State, Multi-Agent Base Branch, Agent Worktrees, Dispatch Decisions, Branch And Path Rules, Merge Strategy, Conflict Rules, +2 more | none | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 |
