# Backend Result: Agent Invocation API

## Summary

Protected agent invocation is implemented through application service and Axum route wiring.

## Evidence Rows

- AC-AIA-01 | red: no protected agent run endpoint existed before this feature. | green: `rtk make be-test` passed authenticated `POST /api/v1/agents/runs` success assertions returning `SUCCESS`, run code, completed status, final output, and step count. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-02 | red: unauthenticated and invalid invocation requests had no stable boundary. | green: `rtk make be-test` passed auth and invalid request assertions for missing prompt, missing workspace, invalid workspace, and missing model config. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-03 | red: invocation could not freeze workspace, user, app, variables, skills, model, tools, and limits before runtime execution. | green: `rtk make be-test` passed invocation service assertions proving the runtime context is built from pre-execution snapshots. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-AIA-04 | red: runtime failures had no API error mapping and secret-safety response proof. | green: `rtk make be-test` passed stable error-code and response secret-safety assertions. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/application/agent.rs`
- `tron-backend/src/adapters/inbound/http/agents.rs`
- `tron-backend/src/adapters/inbound/http/agents/`
- `tron-backend/src/app.rs`
- `tron-backend/tests/agent_invocation_api.rs`

## Commands

- `rtk make be-test`
- `rtk make be-check`

## Blockers

None.
