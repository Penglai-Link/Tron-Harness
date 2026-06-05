# Acceptance: MCP Management

## Acceptance Index

- AC-MCP-01 [must] [business] Platform settings persist one MCP gateway singleton and one platform-owned remote MCP server registry with stable create, update, disable, enable, health, and delete behavior; proof: backend tests and final review show persisted state and delete cascade semantics.
- AC-MCP-02 [must] [business] Workspace `/mcp-servers` resolves inherited platform MCP servers for the active workspace, allows only local enable or disable behavior, and never exposes local connection-definition editing in v1; proof: backend tests and browser proof show effective-state behavior and read-only upstream metadata.
- AC-MCP-03 [must] [api] Internal-agent MCP policy resolves `disabled`, `all_enabled_servers`, and `selected_servers`, rejects stale deleted server codes, and keeps the current system-agent API family unchanged; proof: backend tests and final review show policy resolution and stale-assignment behavior.
- AC-MCP-04 [must] [business] Internal-agent runtime preparation materializes effective MCP tools per run with stable `mcp.<server_code>.<tool_name>` naming and future-run-only observation of later MCP configuration changes; proof: backend tests and final review show runtime materialization and snapshot isolation.
- AC-MCP-05 [must] [interaction] `/settings/mcp-servers`, `/mcp-servers`, and `/agents` expose the agreed controls, state labels, and success or failure feedback flows with authenticated API wiring and no unsupported v1 surfaces; proof: frontend build plus browser smoke passed.

## Open Decisions

- None.
