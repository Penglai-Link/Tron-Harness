# Database Result: Agent Management

## Summary

- status: complete

## Evidence Rows

- AC-AGENT-01 | red: there was no dedicated persistence shape for a platform `Main Agent`, so saved instructions would have had nowhere stable to live. | green: `platform_main_agent_configs` plus sqlite repository support now persist the singleton config independently of run history; `rtk make be-test` passed with runtime-consumption coverage in `tron-backend/tests/agent_management_api.rs`. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Save`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Main Agent saved` toast and disabled `Save` button after persistence; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-02 | red: known system-agent rows would have been lost or reset because no registry table preserved discovery facts across rescans. | green: `system_agent_registries` plus rescan upsert logic now preserve known rows, platform policy, and unavailable transitions; `rtk make be-test` passed with missing-command rescan coverage. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Scan`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scanned 2 system agents; 1 unavailable` toast plus `OpenCode` row `Unavailable` and `Read only`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-03 | red: workspace-effective system-agent state had no storage boundary, so local disable overrides could not stay scoped to one workspace. | green: `workspace_system_agent_policies` plus effective projection now keep local disable overrides scoped to one workspace and one agent; `rtk make be-test` passed with workspace policy coverage. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/agents`, click `Disable` on `Codex`, click `Enable` on `Codex`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=status changes to `Disabled` then `Enabled`, policy changes to `Local disable` then `Platform baseline`, toast shows `Codex disabled in` and then `Codex now follows platform policy`, and the table excludes `Main Agent`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.

## Changed Paths

- `tron-backend/migrations/20260529_0022_create_agent_management.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_management_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs`
- `tron-backend/src/adapters/outbound/persistence.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`

## Blockers

- None.
