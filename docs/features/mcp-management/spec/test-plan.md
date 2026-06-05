# Test Plan: MCP Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-06-05

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-MCP-01 | layer: backend | case: platform MCP gateway config and platform-owned MCP server registry persist, update, health-check, and delete correctly | red: gateway state does not persist, server registry routes fail to store fields or health metadata, or deleting a server leaves dependent override or assignment rows behind | green: backend tests prove singleton gateway reads and writes, server registry CRUD, explicit health summary persistence, and delete cascade over workspace overrides and selected-server assignments | interaction: save gateway config -> create platform server -> edit -> health check -> disable/enable -> delete -> verify dependent rows are removed | command_ref: CMD-MCP-BE-TEST.
- AC-MCP-02 | layer: backend | case: workspace effective MCP server resolution applies platform state plus current-workspace local disable override only | red: workspace list omits inherited servers, allows connection-definition editing, or can make a platform-disabled server effectively enabled | green: backend tests prove current-workspace list includes inherited platform servers, local disable persists only in the current workspace, enable removes the local disable override, and platform disable dominates every workspace | interaction: list workspace effective servers -> disable one inherited enabled row -> verify current-workspace state becomes disabled while platform row remains enabled -> re-enable -> disable same server globally -> verify workspace cannot make it effectively enabled | command_ref: CMD-MCP-BE-TEST.
- AC-MCP-03 | layer: backend | case: internal-agent MCP policy resolves access modes and selected-server assignments safely | red: `mcpAccessMode` does not persist, selected-server replacement accepts deleted or no-longer-effective server codes, or MCP policy mutates current system-agent APIs | green: backend tests prove `disabled`, `all_enabled_servers`, and `selected_servers` resolution, stale deleted server codes are rejected, selected-server replacement persists by server, and existing system-agent endpoints remain behaviorally unchanged | interaction: read internal-agent policy -> patch MCP access mode -> replace selected servers -> delete one referenced server -> retry stale submission -> verify typed stale error and refreshed effective policy | command_ref: CMD-MCP-BE-TEST.
- AC-MCP-04 | layer: backend | case: runtime materializes MCP tools from effective servers and keeps run snapshots stable | red: internal-agent invocation omits eligible MCP tools, tool names do not use the `mcp.<server_code>.<tool_name>` contract, or later MCP mutations retroactively alter an active run | green: backend tests prove runtime resolves eligible servers, materializes correctly named MCP tools into the run context, and later disable/delete changes affect only future runs while current runs keep their snapshot | interaction: prepare internal-agent run with MCP access enabled -> inspect materialized runtime tools -> change workspace or platform server state -> prepare second run -> verify first and second runs differ only where expected | command_ref: CMD-MCP-BE-TEST.
- AC-MCP-05 | layer: browser | case: platform settings, workspace MCP page, and `/agents` internal-agent MCP controls render agreed interactions and feedback | red: `/settings/mcp-servers` lacks gateway or registry controls, `/mcp-servers` exposes unsupported edit fields, `/agents` cannot manage MCP access mode, or success/error feedback is missing | green: browser smoke proves gateway save, platform registry create/edit/disable/delete affordances, workspace enable/disable behavior, internal-agent MCP access mode and selected-server flows, and visible state/feedback transitions | interaction: open `/settings/mcp-servers` -> save gateway config -> create or edit one server -> open `/mcp-servers` -> disable then re-enable one inherited row -> open `/agents` -> change MCP access mode and selected servers -> verify feedback and state labels | command_ref: CMD-MCP-BROWSER.

## Command Index

- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: backend MCP management code compiles cleanly.
- CMD-MCP-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify/final | rerun: affected backend, persistence, or runtime changes | expect: backend MCP management tests pass along with the existing backend test suite.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: MCP management pages compile and build.
- CMD-MCP-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test mcp-management-smoke.spec.ts --reporter=line | cost: slow/browser | when: verify/final | rerun: affected settings, workspace, or internal-agent MCP UI changes | expect: `/settings/mcp-servers`, `/mcp-servers`, and `/agents` match the accepted interactions.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/mcp-management --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: MCP management feature docs validate end to end.

## Evidence Rules

- Every `AC-MCP-*` row must have final evidence in reports.
- Backend evidence for AC-MCP-01 must explicitly mention gateway persistence, health summary persistence, and delete cascade semantics.
- Backend evidence for AC-MCP-02 must explicitly mention that workspace pages cannot edit upstream connection definitions in v1.
- Backend evidence for AC-MCP-03 must explicitly mention stale deleted-server rejection and unchanged system-agent behavior.
- Backend evidence for AC-MCP-04 must explicitly mention `mcp.<server_code>.<tool_name>` naming and future-run-only observation of later config changes.
- Browser evidence for AC-MCP-05 must list actual clicks, any selected inputs, visible feedback, and relevant network assertions across all three surfaces.
