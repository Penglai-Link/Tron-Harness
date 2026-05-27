# Review Report: Agent Invocation API

## Summary

Review confirmed the invocation API uses the existing auth middleware, uniform response envelope, application service boundary, and runtime kernel.

## Evidence Rows

- AC-AIA-01 | red: review found no pre-existing agent invocation route. | green: review confirmed `POST /api/v1/agents/runs` is merged under protected routes and returns `ApiResponse` data. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-AIA-02 | red: review found no pre-existing validation boundary for agent runs. | green: review confirmed route extraction requires auth context and application validation rejects invalid prompt, workspace, and model input. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-AIA-03 | red: review found runtime context assembly was absent before the service. | green: review confirmed the service snapshots workspace, user, app, variables, skills, model config, tools, and limits before calling `ReactAgent`. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-AIA-04 | red: review found no stable API mapping for runtime errors. | green: review confirmed `error_mapper.rs` maps agent errors to stable codes and response DTOs do not include API keys or sensitive event payloads. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-AIA-05 | red: review required live app-shell proof after backend route wiring. | green: review reused the fresh browser verification artifact plus passing frontend build. | command_ref: CMD-FE-CHECK | browser: clicks=clicked Development OAuth2 provider, landed on protected agent route, clicked Apps navigation, returned to agent route; inputs=filled login field with auth operator, filled password field, filled first visible protected-route field with agent runtime; network=`GET /api/v1/auth/providers` 200, `GET /api/v1/auth/oauth2/authorize/dev` 303, `GET /api/v1/auth/oauth2/callback/dev` 303, `POST /api/v1/auth/refresh` 200 after callback; feedback=protected agent route and Apps page rendered, final route showed Main Agent content; artifacts=`docs/features/agent-invocation-api/reports/browser-agent-api-shell.png`, `docs/features/agent-invocation-api/reports/browser-agent-api-network.json` | status: done.

## Changed Paths

- `tron-backend/src/application/agent.rs`
- `tron-backend/src/adapters/inbound/http/agents/error_mapper.rs`
- `tron-backend/src/adapters/inbound/http/agents/handler.rs`
- `tron-backend/src/adapters/inbound/http/agents/routes.rs`
- `tron-backend/src/app.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`
- `rtk make fe-check`

## Blockers

None.
