# Test Report: Agent Invocation API

## Summary

Backend, frontend, and browser verification all passed for the invocation API feature.

## Evidence Rows

- AC-AIA-01 | red: before implementation, compiling a test for `POST /api/v1/agents/runs` failed because no route or application service existed. | green: `rtk make be-test` passed authenticated invocation assertions with uniform success body. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-02 | red: invalid or unauthenticated invocation behavior was undefined before route wiring. | green: `rtk make be-test` passed rejection assertions for auth and input validation. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-03 | red: invocation snapshots were absent before application service implementation. | green: `rtk make be-test` passed frozen context assertions for workspace, user, app, variables, skills, model, tools, and limits. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-04 | red: runtime error mapping and response secret-safety were absent before implementation. | green: `rtk make be-test` passed stable error and no-credential-leak assertions. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-05 | red: `rtk make fe-check` first failed because frontend packages were not installed in the worktree. | green: after `rtk make fe-install`, `rtk make fe-check` passed and browser verification passed against local services. | command_ref: CMD-FE-CHECK | browser: clicks=clicked Development OAuth2 provider, landed on protected agent route, clicked Apps navigation, returned to agent route; inputs=filled login field with auth operator, filled password field, filled first visible protected-route field with agent runtime; network=`GET /api/v1/auth/providers` 200, `GET /api/v1/auth/oauth2/authorize/dev` 303, `GET /api/v1/auth/oauth2/callback/dev` 303, `POST /api/v1/auth/refresh` 200 after callback; feedback=Main Agent was visible on protected route, Apps page heading was visible, final protected agent route remained usable; artifacts=`docs/features/agent-invocation-api/reports/browser-agent-api-shell.png`, `docs/features/agent-invocation-api/reports/browser-agent-api-network.json` | status: done.

## Changed Paths

- `tron-backend/tests/agent_invocation_api.rs`
- `tron-backend/src/application/agent.rs`
- `tron-backend/src/adapters/inbound/http/agents/`
- `docs/features/agent-invocation-api/reports/browser-agent-api-shell.png`
- `docs/features/agent-invocation-api/reports/browser-agent-api-network.json`

## Commands

- `rtk make be-test`
- `rtk make be-check`
- `rtk make fe-install`
- `rtk make fe-check`
- `rtk node --input-type=module`

## Blockers

None.
