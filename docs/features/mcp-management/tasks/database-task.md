# Database Task: MCP Management

## Execution Contract

- feature_slug: mcp-management
- status: complete
- execution_mode: hybrid

## TDD Contract

Database rows use bounded AC slices with RED before GREEN and focused VERIFY.

## Task Rows

- AC-MCP-01 | phase: tdd | owner: backend-worker | scope: add SQLite migrations and repository support for the gateway singleton, platform server registry, health summary fields, and workspace override delete cascade | command_ref: CMD-MCP-BE-TEST | status: done.
- AC-MCP-03 | phase: tdd | owner: backend-worker | scope: extend internal-agent policy storage with `mcp_access_mode` and selected-server assignment persistence keyed by `(workspace_code, agent_code, server_code)` | command_ref: CMD-MCP-BE-TEST | status: done.

## Worktree Notes

- Database changes stay inside the backend worker scope because migrations, repositories, and policy mapping overlap heavily with backend service ownership.

## Integration Notes

- New migrations must be registered in `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`.
- Platform delete must cascade workspace overrides and selected-server assignments through both SQL and repository behavior.

## Open Decisions

- None.
