# Test Report: Agent AG-UI Stream Adapter

## Summary

Feature 4 backend adapter, safety, frontend build, and browser interaction ACs have executable evidence.

## Evidence Rows

- AC-AAS-01 | red: `agent_agui_stream_api` first failed with 404 because stream activation and replay routes were absent | green: authenticated stream activation and deactivation return stream state through protected routes | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AAS-02 | red: unified agent events had no AG-UI-shaped replay DTO endpoint | green: replay conversion returns run, model, and output event snapshots with stream key, run identity, type, outcome, timestamp, and metadata | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AAS-03 | red: adapter safety assertions had no converted JSON to inspect | green: converted event JSON excludes prompt, completion text, raw tool output, API key, token, and request body values | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AAS-04 | red: route wiring and browser app-shell usability required validation after backend changes | green: `rtk make be-test`, `rtk make fe-check`, and browser automation succeeded | command_ref: CMD-FE-CHECK | browser: clicks=OAuth2 provider button, Apps sidebar link, Assistant topbar button; inputs=Apps search=agent; network=providers 200, authorize 303, callback 303, refresh 200; feedback=agent page visible, App Management visible, assistant panel visible; artifacts=browser-agent-feature34-shell-png and browser-agent-feature34-network-json | status: done.

## Changed Paths

- `tron-backend/src/adapters/inbound/http/agents/handler.rs`
- `tron-backend/src/adapters/inbound/http/agents/routes.rs`
- `tron-backend/src/application/agent.rs`
- `tron-backend/src/domain/agent.rs`
- `tron-backend/tests/agent_agui_stream_api.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`
- `rtk make fe-check`
- Browser automation against `http://127.0.0.1:5173/agent`

## Blockers

- None.
