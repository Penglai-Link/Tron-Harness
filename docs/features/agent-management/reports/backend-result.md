# Backend Result: Agent Management

## Summary

- status: complete

## Evidence Rows

- AC-AGENT-01 | red: saved platform `Main Agent` fields were not injected into runtime requests, so settings would have stored dead configuration. | green: `PlatformMainAgentConfig` persistence plus `AgentInvocationService` guidance injection now land under `tron-backend/src/application/agent/*`, `tron-backend/src/domain/agent/*`, and `tron-backend/src/adapters/inbound/http/agents/*`; `rtk make be-test` passed with `tron-backend/tests/agent_management_api.rs` proving saved `personality` and `instructions` are injected only for main-agent runs. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Save`; inputs=append validation text to `Personality` and `Instructions`; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Main Agent saved` toast and disabled `Save` button after persistence; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-02 | red: startup scan and rescan could not preserve policy or keep missing system agents visible because no registry lifecycle existed. | green: startup scan and manual rescan now persist known system-agent rows, preserve platform enable or disable policy, and mark missing commands `UNAVAILABLE`; `rtk make be-test` passed with router-level coverage in `tron-backend/tests/agent_management_api.rs`. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/settings/agents`, click `Scan`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=`Scanned 2 system agents; 1 unavailable` toast plus `OpenCode` row `Unavailable` and `Read only`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.
- AC-AGENT-03 | red: workspace `/agents` could not resolve inherited platform rows or keep platform-disable dominance because the backend lacked effective-policy endpoints. | green: workspace `/api/v1/agents/workspaces/{workspaceCode}/system` now excludes `Main Agent`, resolves inherited platform rows, supports local disable or enable semantics, and keeps platform disable dominant; `rtk make be-test` passed with router-level coverage in `tron-backend/tests/agent_management_api.rs`. | command_ref: CMD-AGENT-BE-TEST | browser: clicks=open `/agents`, click `Disable` on `Codex`, click `Enable` on `Codex`; inputs=none; network=no `/api/v1/**` request escaped the blocked Playwright harness; feedback=status changes to `Disabled` then `Enabled`, policy changes to `Local disable` then `Platform baseline`, toast shows `Codex disabled in` and then `Codex now follows platform policy`, and the table excludes `Main Agent`; artifacts=`tron-frontend/tests/agent-management-smoke.spec.ts`, `Playwright 1 passed`. | status: done.

## Changed Paths

- `tron-backend/src/app.rs`
- `tron-backend/src/application/agent.rs`
- `tron-backend/src/application/agent/management.rs`
- `tron-backend/src/application/agent/invocation.rs`
- `tron-backend/src/domain/agent.rs`
- `tron-backend/src/domain/agent/management.rs`
- `tron-backend/src/domain/agent/message.rs`
- `tron-backend/src/adapters/inbound/http/agents.rs`
- `tron-backend/src/adapters/inbound/http/agents/routes.rs`
- `tron-backend/src/adapters/inbound/http/agents/handler.rs`
- `tron-backend/src/adapters/inbound/http/agents/error_mapper.rs`
- `tron-backend/src/adapters/inbound/http/agents/request.rs`
- `tron-backend/src/adapters/inbound/http/chats/handler.rs`
- `tron-backend/src/adapters/outbound/persistence.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_management_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/config.rs`
- `tron-backend/migrations/20260529_0022_create_agent_management.sql`
- `tron-backend/tests/agent_management_api.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`

## Blockers

- None.
