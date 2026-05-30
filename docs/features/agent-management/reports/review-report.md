# Review Report: Agent Management

## Summary

- status: complete

## Evidence Rows

- AC-AGENT-01 | red: the first frontend or backend pass allowed the feature to look complete while the saved main-agent config still risked being dead runtime data. | green: final review confirmed persisted main-agent guidance is injected only for main-agent runs and that the settings page exposes the exact saved fields; `rtk make be-check`, `rtk make be-test`, and Playwright all passed. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Save`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Main Agent saved` toast and disabled `Save` button after persistence; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-02 | red: the initial frontend implementation assumed a rescan payload shape that the backend did not actually return. | green: review closed that contract drift by making the backend return counts plus refreshed rows and by normalizing real API values in the frontend before rendering; backend and browser verification both passed. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Scan`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scanned 2 system agents; 1 unavailable` toast plus `OpenCode` row `Unavailable` and `Read only`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-03 | red: the first workspace action logic exposed a misleading `Enable` button for rows already following a platform-disabled baseline. | green: review tightened workspace action semantics so only local overrides or platform-enabled rows are mutable from workspace scope; backend and Playwright verification both passed. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/agents`, click `Disable` on `Codex`, click `Enable` on `Codex`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=status changes to `Disabled` then `Enabled`, policy changes to `Local disable` then `Platform baseline`, toast shows `Codex disabled in` and then `Codex now follows platform policy`, and the table excludes `Main Agent`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-04 | red: browser verification in blocked-request mode could have passed while real API casing or rescan payload differences still broke the live UI. | green: review verified route wiring, API normalization, mock bounded scope, and no-archive action rules together; `rtk make fe-check` and Playwright passed after the fixes. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/agents`, open `/settings/agents`, click `Scan`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scan` visible, no archive buttons rendered, and unavailable rows show `Read only` without buttons; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.

## Findings

- None. Final review found and fixed the rescan payload mismatch, stale unsupported mock agent, and misleading workspace `Enable` action before closeout.

## Changed Paths

- `tron-backend/src/application/agent/management.rs`
- `tron-backend/src/domain/agent/management.rs`
- `tron-frontend/src/domain/agent/api/agentManagementApi.ts`
- `tron-frontend/src/domain/agent/mock/managementData.ts`
- `tron-frontend/src/domain/agent/pages/WorkspaceAgentsPage.tsx`
- `tron-frontend/tests/agent-management-smoke.spec.ts`
- `docs/features/agent-management/reports/review-report.md`

## Commands

- `rtk make be-check`
- `rtk make fe-check`
- `rtk make be-test`
- `rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line`

## Residual Risks

- `en-US` is still the only shipped locale in the frontend tree, which matches the current repository reality but not the broader project note about future `zh-CN` readiness.

## Blockers

- None.
