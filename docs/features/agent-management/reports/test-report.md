# Test Report: Agent Management

## Summary

- status: complete

## Evidence Rows

- AC-AGENT-01 | red: before integration, saving main-agent fields would have changed UI draft state only because runtime requests still omitted persisted guidance. | green: `rtk make be-test` passed with runtime request recording, and `rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line` passed with visible save feedback on `/settings/agents`. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Save`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Main Agent saved` toast and disabled `Save` button after persistence; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-02 | red: scan behavior could not prove row preservation or unavailable transitions because the UI and backend contracts were disconnected. | green: `rtk make be-test` passed with rescan-policy coverage, and Playwright passed with the `Scan` flow showing a refreshed registry and unavailable row after scan. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Scan`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scanned 2 system agents; 1 unavailable` toast plus `OpenCode` row `Unavailable` and `Read only`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-03 | red: `/agents` did not exist and workspace-local disable or enable semantics were therefore untestable. | green: `rtk make be-test` passed with workspace override coverage, and Playwright passed with `Codex` toggling between `Platform baseline` and `Local disable` while `Main Agent` stayed hidden. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/agents`, click `Disable` on `Codex`, click `Enable` on `Codex`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=status changes to `Disabled` then `Enabled`, policy changes to `Local disable` then `Platform baseline`, toast shows `Codex disabled in` and then `Codex now follows platform policy`, and the table excludes `Main Agent`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-04 | red: route wiring, `Scan`, and action rendering had no end-to-end proof and real API normalization bugs were still possible. | green: `rtk make fe-check` and the focused Playwright path both passed after route wiring, API normalization, and action-state fixes. | command_ref: CMD-AGENT-BROWSER | browser: clicks=open `/agents`, open `/settings/agents`, click `Scan`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scan` visible, no archive buttons rendered, and unavailable rows show `Read only` without buttons; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.

## Commands

- `rtk make be-check`
- `rtk make fe-check`
- `rtk make be-test`
- `rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line`

## Changed Paths

- `tron-backend/tests/agent_management_api.rs`
- `tron-frontend/tests/agent-management-smoke.spec.ts`
- `docs/features/agent-management/reports/test-report.md`

## Blockers

- None.
