# Backend Result: MCP Management

## Summary

- Backend MCP gateway, platform registry, workspace-effective server policy, internal-agent MCP policy, and runtime MCP tool materialization are implemented.
- Feature-level backend tests and the full backend suite both passed on 2026-06-05.

## Evidence Rows

- AC-MCP-01 | red: there was no MCP gateway singleton, no platform-owned MCP registry, and no dedicated MCP route family. | green: `rtk make be-test` passed with `mcp_management_api` proving gateway persistence, platform registry CRUD, health refresh, and delete cascade over dependent workspace and agent rows. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_contract_focus; inputs=gateway url auth headers plus platform server create edit disable enable and delete payloads in API tests; network=`GET/PUT /api/v1/mcp/gateway-config`, `GET/POST/PATCH/DELETE /api/v1/mcp/servers`, and `POST /api/v1/mcp/servers/{serverCode}/health-check`; feedback=uniform success envelopes persisted gateway fields, refreshed health summaries, and removed dependent rows after delete; artifacts=`tron-backend/tests/mcp_management_api.rs` | status: done.
- AC-MCP-02 | red: workspaces had no inherited MCP projection and no local enable or disable override API. | green: `rtk make be-test` passed with `mcp_management_api` proving inherited rows, current-workspace disable persistence, re-enable removal of the local override, and platform-disable dominance. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_contract_focus; inputs=workspace code plus server disable and enable requests in API tests; network=`GET /api/v1/mcp/workspaces/current/servers`, `POST /api/v1/mcp/workspaces/current/servers/{serverCode}/disable`, and `DELETE /api/v1/mcp/workspaces/current/servers/{serverCode}/disable`; feedback=workspace effective rows changed between enabled and disabled while platform state remained authoritative; artifacts=`tron-backend/tests/mcp_management_api.rs` | status: done.
- AC-MCP-03 | red: internal-agent policy had no `mcpAccessMode`, no selected-server assignment persistence, and no stale deleted-server rejection. | green: `rtk make be-test` passed with `agent_management_api` proving `DISABLED`, `ALL_ENABLED_SERVERS`, and `SELECTED_SERVERS`, persisted selected servers by server code, and rejected deleted server submissions with `AGENT_INTERNAL_MCP_SERVER_ASSIGNMENT_STALE`. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_contract_focus; inputs=internal-agent policy patch payloads for `mcpAccessMode` and `selectedServerCodes`; network=`GET/PATCH /api/v1/workspaces/current/agents/internal` plus platform server delete before the stale retry; feedback=policy reads returned refreshed selected-server state and stale retries returned the typed error code without changing system-agent behavior; artifacts=`tron-backend/tests/agent_management_api.rs` | status: done.
- AC-MCP-04 | red: internal-agent runtime did not discover or execute MCP tools and later MCP config changes had no run-scoped snapshot semantics. | green: `rtk make be-test` passed with `agent_management_api` proving runtime materializes `mcp.<server_code>.<tool_name>` tools, executes the remote MCP tool, and keeps an in-flight run usable after later platform disable while the next run drops the disabled MCP tool. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_runtime_test_focus; inputs=agent run request plus mocked search-docs MCP tool call; network=`POST /api/v1/agents/runs` together with MCP JSON-RPC `tools/list` and `tools/call` against the test server; feedback=the first run completed successfully after later disable and the second run no longer exposed the MCP tool; artifacts=`tron-backend/tests/agent_management_api.rs` | status: done.

## Changed Paths

- `tron-backend/src/application/mcp/**`
- `tron-backend/src/adapters/inbound/http/mcp/**`
- `tron-backend/src/application/agent/invocation.rs`
- `tron-backend/src/application/agent/invocation/helpers.rs`
- `tron-backend/src/application/agent/runtime_tool_calls.rs`
- `tron-backend/src/application/agent/management.rs`
- `tron-backend/src/application/agent/management/internal_agents.rs`
- `tron-backend/src/adapters/inbound/http/agents/**`
- `tron-backend/src/adapters/outbound/persistence/sqlite/mcp_repository.rs`
- `tron-backend/tests/mcp_management_api.rs`
- `tron-backend/tests/agent_management_api.rs`

## Commands

- `rtk make be-check` passed on 2026-06-05.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test mcp_management_api -- --nocapture` passed on 2026-06-05.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test agent_management_api -- --nocapture` passed on 2026-06-05.
- `rtk make be-test` passed on 2026-06-05.

## Blockers

- None.
