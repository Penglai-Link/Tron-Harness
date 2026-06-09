# Test Report: MCP Management

## Summary

- The MCP management proof plan is covered by backend feature tests, the full backend suite, frontend build and typecheck, and a dedicated Playwright smoke.
- All AC rows now have fresh GREEN or VERIFY evidence from 2026-06-05.

## Evidence Rows

- AC-MCP-01 | red: no MCP gateway or platform registry existed. | green: `rtk make be-test` passed with `mcp_management_api` proving gateway persistence, registry CRUD, health refresh, and delete semantics. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_contract_focus; inputs=gateway config plus platform server CRUD and health-check payloads in API tests; network=`/api/v1/mcp/gateway-config`, `/api/v1/mcp/servers`, and `/api/v1/mcp/servers/{serverCode}/health-check` completed with persisted state transitions; feedback=stored gateway and registry state reloaded correctly and delete removed dependent rows; artifacts=`tron-backend/tests/mcp_management_api.rs` | status: done.
- AC-MCP-02 | red: no workspace inherited MCP server projection existed. | green: `rtk make be-test` plus the dedicated MCP smoke proved effective state, local disable or enable behavior, and the absence of unsupported local authoring fields. | command_ref: CMD-MCP-BE-TEST | browser: clicks=load `/mcp-servers`, disable one row, enable the same row; inputs=none_form_focus on the inherited workspace page; network=effective-server load plus disable and enable requests succeeded and matched backend policy rules; feedback=effective-state badges changed correctly and no local url or auth editors appeared; artifacts=`tron-backend/tests/mcp_management_api.rs`, `tron-frontend/tests/mcp-management-smoke.spec.ts` | status: done.
- AC-MCP-03 | red: internal-agent MCP assignment did not persist or reject stale rows. | green: `rtk make be-test` proved policy persistence, selected-server replacement, and typed stale rejection for deleted server codes while leaving the system-agent API family unchanged. | command_ref: CMD-MCP-BE-TEST | browser: clicks=load `/agents`, switch to selected-server mode, save the policy, retry after deleting a referenced server in backend tests; inputs=`mcpAccessMode=SELECTED_SERVERS` and one selected server code; network=internal-agent read and patch requests succeeded before the stale retry and the stale retry returned `AGENT_INTERNAL_MCP_SERVER_ASSIGNMENT_STALE`; feedback=the saved policy reloaded correctly and stale retries preserved the refreshed surviving selection only; artifacts=`tron-backend/tests/agent_management_api.rs`, `tron-frontend/tests/mcp-management-smoke.spec.ts` | status: done.
- AC-MCP-04 | red: runtime invocation omitted MCP tools and had no snapshot isolation. | green: `rtk make be-test` proved `mcp.<server_code>.<tool_name>` naming, remote execution, and future-run-only observation of later platform disable changes. | command_ref: CMD-MCP-BE-TEST | browser: clicks=none_runtime_test_focus; inputs=agent run request and mocked search-docs MCP tool call; network=agent run API plus MCP JSON-RPC `tools/list` and `tools/call` requests completed in the test harness; feedback=the first run kept its snapshot after later disable and the second run dropped the MCP tool; artifacts=`tron-backend/tests/agent_management_api.rs` | status: done.
- AC-MCP-05 | red: the agreed MCP pages and feedback flows did not exist. | green: `rtk make fe-check` and the dedicated Playwright MCP smoke proved `/settings/mcp-servers`, `/mcp-servers`, and `/agents` interactions in mock mode. | command_ref: CMD-MCP-BROWSER | browser: clicks=open `/settings/mcp-servers`, save gateway config, create or edit a server, open `/mcp-servers`, disable and enable one row, open `/agents`, save MCP access mode; inputs=gateway url, platform server metadata, and internal-agent MCP mode values; network=settings, workspace, and internal-agent requests all succeeded in the smoke run; feedback=toasts, badges, and selected-state labels stayed aligned with each action across all three surfaces; artifacts=`tron-frontend/tests/mcp-management-smoke.spec.ts` | status: done.

## Changed Paths

- `tron-backend/tests/mcp_management_api.rs`
- `tron-backend/tests/agent_management_api.rs`
- `tron-frontend/tests/mcp-management-smoke.spec.ts`
- `docs/features/mcp-management/spec/test-plan.md`
- `docs/features/mcp-management/reports/*.md`

## Commands

- `rtk make be-check` passed on 2026-06-05.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test mcp_management_api -- --nocapture` passed on 2026-06-05.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test agent_management_api -- --nocapture` passed on 2026-06-05.
- `rtk make be-test` passed on 2026-06-05.
- `rtk make fe-install` passed on 2026-06-05.
- `rtk make fe-check` passed on 2026-06-05.
- `rtk npm --prefix tron-frontend exec playwright -- test mcp-management-smoke.spec.ts --reporter=line` passed `1/1` on 2026-06-05.

## Blockers

- None.
