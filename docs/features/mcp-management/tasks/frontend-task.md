# Frontend Task: MCP Management

## Execution Contract

- feature_slug: mcp-management
- status: complete
- execution_mode: hybrid

## TDD Contract

Frontend rows use bounded AC slices with RED before GREEN and focused VERIFY.

## Task Rows

- AC-MCP-02 | phase: tdd | owner: frontend-worker | scope: add workspace `/mcp-servers` page with effective inherited server state, source ownership, and local enable/disable actions only | command_ref: CMD-MCP-BROWSER | status: done.
- AC-MCP-03 | phase: tdd | owner: frontend-worker | scope: extend workspace `/agents` internal-agent section with MCP access mode and selected-server assignment UI backed by authenticated localized API adapters | command_ref: CMD-MCP-BROWSER | status: done.
- AC-MCP-05 | phase: tdd | owner: frontend-worker | scope: add `/settings/mcp-servers` platform page, route wiring, settings navigation, requestApi adapters, and browser smoke coverage across settings/workspace/agents surfaces | command_ref: CMD-MCP-BROWSER | status: done.

## Worktree Notes

- Lead branch: `feature/clouds3n/mcp-management/base`.
- Planned worker branch: `feature/clouds3n/mcp-management/frontend/pages-and-browser`.

## Integration Notes

- Frontend worker owns `tron-frontend/**` MCP pages, route wiring, API adapters, mocks, and browser tests.
- Protected/localized requests should reuse `requestApi` so `Authorization` and `Accept-Language` both flow through.
- Workspace pages must avoid unsupported v1 authoring fields such as local URL/auth/header editing.

## Open Decisions

- None.
