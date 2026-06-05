# Backend Task: MCP Management

## Execution Contract

- feature_slug: mcp-management
- status: in_progress
- execution_mode: hybrid

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY.

## Task Rows

- AC-MCP-01 | phase: tdd | owner: backend-worker | scope: add platform MCP gateway singleton persistence, platform server registry CRUD, health summary flows, and delete cascade behavior plus backend tests | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-02 | phase: tdd | owner: backend-worker | scope: add workspace effective MCP server projection and current-workspace enable/disable override APIs with platform-disable dominance validation | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-03 | phase: tdd | owner: backend-worker | scope: extend internal-agent MCP policy persistence and API flows with `mcpAccessMode`, selected-server replacement, and stale deleted-server rejection while preserving current system-agent behavior | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-04 | phase: tdd | owner: backend-worker | scope: bridge effective MCP servers into runtime discovery/materialization, register executable MCP tool paths, and prove run-snapshot isolation for later config changes | command_ref: CMD-MCP-BE-TEST | status: ready.

## Worktree Notes

- Lead branch: `feature/clouds3n/mcp-management/base`.
- Planned worker branch: `feature/clouds3n/mcp-management/backend/registry-runtime`.

## Integration Notes

- Backend worker owns `tron-backend/**` for MCP registry, policy, router, runtime, and test changes.
- Runtime materialization must satisfy the current `ToolRegistry` execution gate rather than bypassing it.
- Management routes stay auth-only in v1 and must not silently introduce route-level permission checks.

## Open Decisions

- None.
