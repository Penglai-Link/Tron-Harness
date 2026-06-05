# Test Task: MCP Management

## Execution Contract

- feature_slug: mcp-management
- status: in_progress
- execution_mode: hybrid

## TDD Contract

Verify rows reuse fresh GREEN evidence where possible and rerun slow/browser commands only when required.

## Task Rows

- AC-MCP-01 | phase: verify | owner: lead | scope: verify gateway persistence, platform registry health behavior, and delete-cascade evidence after integration | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-02 | phase: verify | owner: lead | scope: verify workspace effective MCP server evidence and platform-disable dominance after integration | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-03 | phase: verify | owner: lead | scope: verify internal-agent MCP policy resolution, selected-server replacement, and stale-assignment behavior after integration | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-04 | phase: verify | owner: lead | scope: verify runtime materialization naming and future-run-only observation of later MCP mutations | command_ref: CMD-MCP-BE-TEST | status: ready.
- AC-MCP-05 | phase: verify | owner: frontend-worker | scope: verify browser interactions and visible feedback across `/settings/mcp-servers`, `/mcp-servers`, and `/agents` | command_ref: CMD-MCP-BROWSER | status: ready.

## Worktree Notes

- Lead owns final verification judgment and decides whether existing GREEN evidence is still fresh enough for final review.

## Integration Notes

- Runtime verification must mention `mcp.<server_code>.<tool_name>` naming and future-run-only observation of later config changes.
- Browser verification must mention all three surfaces explicitly.

## Open Decisions

- None.
