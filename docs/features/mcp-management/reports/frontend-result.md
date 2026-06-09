# Frontend Result: MCP Management

## Summary

- The frontend now exposes `/settings/mcp-servers`, `/mcp-servers`, and internal-agent MCP controls inside `/agents`.
- Both `en-US` and `zh-CN` locale surfaces were updated and the dedicated MCP smoke passed.

## Evidence Rows

- AC-MCP-02 | red: there was no workspace MCP page and no inherited effective-server interaction. | green: the dedicated MCP smoke proved `/mcp-servers` renders inherited platform servers, effective state, and local enable or disable actions only. | command_ref: CMD-MCP-BROWSER | browser: clicks=open `/mcp-servers`, click disable on one inherited row, click enable on the same row; inputs=none_form_focus on the workspace page; network=workspace effective-server load plus disable and enable requests completed successfully; feedback=state badges and action affordances changed between disabled and enabled without exposing local url or auth editing; artifacts=`tron-frontend/tests/mcp-management-smoke.spec.ts` | status: done.
- AC-MCP-03 | red: `/agents` had no MCP access-mode or selected-server controls for internal agents. | green: the MCP smoke proved `/agents` now exposes `mcpAccessMode`, selected-server assignment, and successful save feedback backed by authenticated localized adapters. | command_ref: CMD-MCP-BROWSER | browser: clicks=open `/agents`, choose `Selected Servers`, pick one server, save the policy; inputs=agent MCP mode value `SELECTED_SERVERS` and selected server codes; network=internal-agent policy read and patch requests completed with authenticated localized headers; feedback=success toast appeared and the selected-server list reloaded with the saved state; artifacts=`tron-frontend/tests/mcp-management-smoke.spec.ts` | status: done.
- AC-MCP-05 | red: there was no platform MCP settings surface or end-to-end MCP UI smoke. | green: `rtk make fe-check` and the dedicated MCP smoke proved gateway save, platform registry create or edit or disable flows, workspace effective-server interactions, and internal-agent MCP controls across all three surfaces. | command_ref: CMD-MCP-BROWSER | browser: clicks=open `/settings/mcp-servers`, save gateway config, create server, edit server, disable server, open `/mcp-servers`, open `/agents`; inputs=gateway url and platform server name or code or url plus internal-agent MCP mode values; network=gateway config, platform registry, workspace effective-server, and internal-agent policy requests all succeeded; feedback=visible state labels, save toasts, and action buttons stayed aligned across settings, workspace, and agents pages; artifacts=`tron-frontend/tests/mcp-management-smoke.spec.ts` | status: done.

## Changed Paths

- `tron-frontend/src/domain/mcp/**`
- `tron-frontend/src/domain/agent/components/InternalAgentMcpServerSelector.tsx`
- `tron-frontend/src/domain/agent/api/agentApi.ts`
- `tron-frontend/src/domain/agent/api/agentManagementApi.ts`
- `tron-frontend/src/domain/agent/mock/managementData.ts`
- `tron-frontend/src/domain/agent/model/AgentManagementModel.ts`
- `tron-frontend/src/domain/agent/pages/WorkspaceAgentsPage.tsx`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/src/i18n/locales/en-US/pages.ts`
- `tron-frontend/src/i18n/locales/en-US/shell.ts`
- `tron-frontend/src/i18n/locales/zh-CN/pages.ts`
- `tron-frontend/src/i18n/locales/zh-CN/shell.ts`
- `tron-frontend/tests/mcp-management-smoke.spec.ts`

## Commands

- `rtk make fe-install` passed on 2026-06-05.
- `rtk make fe-check` passed on 2026-06-05.
- `rtk npm --prefix tron-frontend exec playwright -- test mcp-management-smoke.spec.ts --reporter=line` passed `1/1` on 2026-06-05.

## Blockers

- None.
