# Database Result: MCP Management

## Summary

- Dedicated MCP SQLite storage and internal-agent MCP assignment storage are implemented under explicit migrations.
- Delete cascade semantics now cover workspace MCP overrides and internal-agent selected-server assignments.

## Evidence Rows

- AC-MCP-01 | red: no MCP gateway or platform or workspace-policy tables existed. | green: `rtk make be-test` passed after adding the gateway, platform server, and workspace policy migrations and proving persisted CRUD plus delete cascade behavior. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_persistence_focus; inputs=gateway fields, platform server rows, and workspace policy mutations exercised in API tests; network=SQLite writes and reads behind `/api/v1/mcp/gateway-config`, `/api/v1/mcp/servers`, and workspace effective-server routes; feedback=stored gateway and server state reloaded correctly and dependent workspace policy rows disappeared after platform delete; artifacts=`tron-backend/migrations/20260605_0001_create_platform_mcp_gateway_configs.sql` | status: done.
- AC-MCP-03 | red: internal-agent policy storage lacked `mcp_access_mode` and selected-server assignment rows. | green: `rtk make be-test` passed after migration `20260605_1001_add_internal_agent_mcp_access_to_workspace_internal_agent_policies.sql` added `mcp_access_mode` plus assignment storage with delete cascade and replacement semantics. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_persistence_focus; inputs=internal-agent MCP policy writes with `selectedServerCodes` replacement and stale retry after server delete; network=SQLite reads and writes behind `/api/v1/workspaces/current/agents/internal` plus platform server delete flows; feedback=policy reloads preserved valid assignments and deleted-server retries produced the typed stale error without leaving dangling rows; artifacts=`tron-backend/migrations/20260605_1001_add_internal_agent_mcp_access_to_workspace_internal_agent_policies.sql` | status: done.

## Changed Paths

- `tron-backend/migrations/20260605_0001_create_platform_mcp_gateway_configs.sql`
- `tron-backend/migrations/20260605_0002_create_platform_mcp_servers.sql`
- `tron-backend/migrations/20260605_0003_create_workspace_mcp_server_policies.sql`
- `tron-backend/migrations/20260605_1001_add_internal_agent_mcp_access_to_workspace_internal_agent_policies.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/mcp_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_management_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_management_repository/connection.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_management_repository/tests.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`

## Commands

- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test mcp_management_api -- --nocapture` passed on 2026-06-05.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test agent_management_api -- --nocapture` passed on 2026-06-05.
- `rtk make be-test` passed on 2026-06-05.

## Blockers

- None.
