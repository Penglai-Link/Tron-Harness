# Frontend Result: Agent Invocation API

## Summary

No new frontend page was required; route wiring remained compatible with the existing shell and auth flow.

## Evidence Rows

- AC-AIA-05 | red: frontend verification initially failed because `tsc` was absent before package install. | green: `rtk make fe-install` completed, `rtk make fe-check` passed, and browser verification completed against running backend and frontend services. | command_ref: CMD-FE-CHECK | browser: clicks=clicked Development OAuth2 provider, landed on protected agent route, clicked Apps navigation, returned to agent route; inputs=filled login field with auth operator, filled password field, filled first visible protected-route field with agent runtime; network=`GET /api/v1/auth/providers` 200, `GET /api/v1/auth/oauth2/authorize/dev` 303, `GET /api/v1/auth/oauth2/callback/dev` 303, `POST /api/v1/auth/refresh` 200 after callback; feedback=agent body showed Main Agent, Apps heading was observed, final route was protected agent route with Main Agent visible; artifacts=`docs/features/agent-invocation-api/reports/browser-agent-api-shell.png`, `docs/features/agent-invocation-api/reports/browser-agent-api-network.json` | status: done.

## Changed Paths

- `docs/features/agent-invocation-api/reports/browser-agent-api-shell.png`
- `docs/features/agent-invocation-api/reports/browser-agent-api-network.json`

## Commands

- `rtk make fe-install`
- `rtk make fe-check`
- `rtk env TRON_AUTH_DEV_OAUTH2_ENABLED=true make be-run`
- `rtk env VITE_TRON_DATA_SOURCE=mock TRON_FRONTEND_API_PROXY_TARGET=http://127.0.0.1:8080 make fe-dev`
- `rtk node --input-type=module`

## Blockers

None.
