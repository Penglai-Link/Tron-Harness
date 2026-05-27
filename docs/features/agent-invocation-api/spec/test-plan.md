# Test Plan: Agent Invocation API

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user delegated decisions to Codex
- approved_at: 2026-05-27

## Proof Rows

- AC-AIA-01 | layer: backend | case: successful protected invocation | red: no protected invocation endpoint returns a uniform run result | green: authenticated API test posts prompt and model config and receives `SUCCESS` with run code, completed status, final output, and step count | command_ref: CMD-BE-TEST.
- AC-AIA-02 | layer: security | case: auth and input rejection | red: unauthenticated or invalid requests can start runs or fail with unstable errors | green: tests prove auth, missing prompt, missing workspace, invalid workspace, and missing model config are rejected with stable codes | command_ref: CMD-BE-TEST.
- AC-AIA-03 | layer: backend | case: frozen invocation snapshot | red: invocation bypasses workspace/user/app/effective data or lets runtime query repositories during execution | green: tests prove response/run context includes workspace, user, app scope, variables, skills, model, tools, and limits captured before runtime | command_ref: CMD-BE-TEST.
- AC-AIA-04 | layer: api | case: runtime error mapping and secret safety | red: runtime failures leak raw API keys or collapse to generic errors | green: tests prove model/tool/limit errors map to stable API codes and responses omit raw credentials | command_ref: CMD-BE-TEST.
- AC-AIA-05 | layer: frontend | case: route wiring regression and browser smoke | red: backend route wiring breaks builds or frontend app smoke | green: `rtk make be-test`, `rtk make fe-check`, and in-app browser clicks through login/app shell without console-breaking errors | interaction: start backend and frontend -> open app -> log in or use available auth path -> click navigation surfaces -> confirm app shell remains usable | command_ref: CMD-FE-CHECK.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red-green-verify | rerun: after backend runtime or API changes | expect: backend tests pass including agent invocation tests.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after backend source changes | expect: backend compiles.
- CMD-FE-CHECK | command: `rtk make fe-check` | cost: medium | when: verify | rerun: after app route wiring or before browser smoke | expect: frontend typecheck and build pass.
- CMD-BROWSER-SMOKE | command: `in-app browser real-click smoke against local frontend/backend` | cost: slow/browser | when: verify | rerun: after frontend/backend route wiring changes or when smoke evidence is stale | expect: app opens and real clicks do not expose broken shell behavior.
- CMD-DOCS-SPEC | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/agent-invocation-api --phase spec --complete-spec --no-downstream-artifacts` | cost: low | when: docs | rerun: after feature spec changes | expect: feature docs spec gate passes.

## Evidence Rules

- API evidence must include actual status codes and response codes.
- Secret-safety evidence must inspect response JSON.
- Browser evidence must list clicks, visible feedback, and artifacts.
- Slow/browser evidence is not rerun during review if fresh and passing.

## Open Decisions

- None.
