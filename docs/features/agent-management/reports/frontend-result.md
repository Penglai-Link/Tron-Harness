# Frontend Result: Agent Management

## Summary

- status: done
- `/settings/agents` now renders a dedicated `Main Agent` editor card plus a system-agent table with `Scan` and enable or disable actions.
- `/agents` now renders inherited system-agent rows only, excludes `Main Agent`, and applies workspace-local enable or disable semantics with platform-disable visibility.
- Frontend mock state, route wiring, navigation entries, and Playwright smoke all cover the new agent-management surface without touching backend files.

## Evidence Rows

- AC-AGENT-01 | red: the frontend only had the `/agent` chat demo, so there was no dedicated platform `Main Agent` editor surface and no clean frontend model for management-specific fields. | green: `/settings/agents` now uses separate management types, API wiring, and mock state to edit `personality` plus `instructions` in a dedicated `Main Agent` card while leaving the `/agent` chat demo untouched; `rtk make fe-check` passed. | command_ref: CMD-FE-CHECK | browser: clicks=open `/settings/agents`, click `Save`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Main Agent saved` toast and disabled `Save` button after persistence; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-02 | red: the frontend had no system-agent registry surface, no `Scan` control, and no normalization for real API status or source fields. | green: `/settings/agents` now exposes a system-agent table plus `Scan`, normalizes real API status or source values, and refreshes rows from the rescan payload that now includes counts plus records; `rtk make fe-check` and focused Playwright passed. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/settings/agents`, click `Scan`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scanned 2 system agents; 1 unavailable` toast plus `OpenCode` row `Unavailable` and `Read only`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-03 | red: `/agents` did not exist, inherited system-agent rows were not modeled separately, and the workspace surface could not show platform inheritance or local disable overrides. | green: `/agents` now renders inherited system agents only, excludes `Main Agent`, shows `Inherited From = Platform`, blocks misleading workspace `Enable` actions when a row is already following a platform-disabled baseline, and supports workspace-local disable then enable flows; `rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line` passed `1 passed`. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/agents`, click `Disable` on `Codex`, click `Enable` on `Codex`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=status changes to `Disabled` then `Enabled`, policy changes to `Local disable` then `Platform baseline`, toast shows `Codex disabled in` and then `Codex now follows platform policy`, and the table excludes `Main Agent`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-04 | red: there was no settings-shell agent management route, no workspace route, no `Scan` action, and no browser proof that system agents never expose archive actions. | green: both `/settings/agents` and `/agents` are now routed and navigable, system-agent rows expose only enable or disable or read-only states, unavailable rows show no action buttons, and the blocked-request Playwright path proves the UI works end to end; `rtk make fe-check` and `rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line` passed. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/agents`, open `/settings/agents`, click `Scan`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scan` visible, no archive buttons rendered, and unavailable rows show `Read only` without buttons; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.

## Changed Paths

- `tron-frontend/src/domain/agent/api/agentApi.ts`
- `tron-frontend/src/domain/agent/api/agentManagementApi.ts`
- `tron-frontend/src/domain/agent/api/index.ts`
- `tron-frontend/src/domain/agent/components/AgentStatusBadge.tsx`
- `tron-frontend/src/domain/agent/components/index.ts`
- `tron-frontend/src/domain/agent/constants/agentManagementOptions.ts`
- `tron-frontend/src/domain/agent/constants/index.ts`
- `tron-frontend/src/domain/agent/mock/index.ts`
- `tron-frontend/src/domain/agent/mock/managementData.ts`
- `tron-frontend/src/domain/agent/model/AgentManagementModel.ts`
- `tron-frontend/src/domain/agent/model/index.ts`
- `tron-frontend/src/domain/agent/pages/PlatformAgentsPage.tsx`
- `tron-frontend/src/domain/agent/pages/WorkspaceAgentsPage.tsx`
- `tron-frontend/src/domain/settings/constants/settingsNavigation.ts`
- `tron-frontend/src/shared/constants/shellNavigation.ts`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/i18n/locales/en-US/pages.ts`
- `tron-frontend/src/i18n/locales/en-US/shell.ts`
- `tron-frontend/tests/agent-management-smoke.spec.ts`
- `docs/features/agent-management/tasks/frontend-task.md`
- `docs/features/agent-management/reports/frontend-result.md`

## Commands

- `rtk make fe-install` completed on 2026-05-29 because the worktree was missing frontend toolchain dependencies.
- `rtk make fe-check` passed on 2026-05-29.
- `rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line` passed with `1 passed` on 2026-05-29.

## Blockers

- None.
