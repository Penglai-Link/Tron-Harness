# Worktree Report: Agent Invocation API

## Summary

Invocation API work stayed on the selected Lite Spec multi-agent base worktree and reused the runtime kernel delivered by the prior feature.

## Evidence Rows

- AC-AIA-01 | red: base worktree lacked the protected invocation route. | green: current worktree contains route, handler, service, DTOs, and passing success tests. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-02 | red: base worktree lacked invocation auth and validation behavior. | green: current worktree contains protected route wiring and stable validation failures with passing tests. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-03 | red: base worktree lacked frozen invocation snapshot assembly. | green: current worktree contains application service snapshot assembly with passing tests. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-04 | red: base worktree lacked runtime error API mapping. | green: current worktree contains agent error mapping with passing tests and review. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-05 | red: worktree browser and frontend verification had no fresh evidence after route wiring. | green: current worktree passed frontend build and browser verification against local backend and frontend services. | command_ref: CMD-FE-CHECK | browser: clicks=clicked Development OAuth2 provider, landed on protected agent route, clicked Apps navigation, returned to agent route; inputs=filled login field with auth operator, filled password field, filled first visible protected-route field with agent runtime; network=`GET /api/v1/auth/providers` 200, `GET /api/v1/auth/oauth2/authorize/dev` 303, `GET /api/v1/auth/oauth2/callback/dev` 303, `POST /api/v1/auth/refresh` 200 after callback; feedback=protected agent route showed Main Agent and navigation to Apps rendered App Management; artifacts=`docs/features/agent-invocation-api/reports/browser-agent-api-shell.png`, `docs/features/agent-invocation-api/reports/browser-agent-api-network.json` | status: done.

## Changed Paths

- `docs/features/agent-invocation-api/`
- `tron-backend/src/application/agent.rs`
- `tron-backend/src/adapters/inbound/http/agents/`
- `tron-backend/tests/agent_invocation_api.rs`

## Commands

- `rtk git status --short --branch`
- `rtk make be-test`
- `rtk make be-check`
- `rtk make fe-check`
- `rtk node --input-type=module`

## Blockers

None.
