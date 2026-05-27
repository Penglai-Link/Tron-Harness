# Test Plan: Agent AG-UI Stream Adapter

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user delegated decisions to Codex
- approved_at: 2026-05-27

## Proof Rows

- AC-AAS-01 | layer: backend | case: stream lifecycle | red: no protected API activates or deactivates AG-UI replay watchers | green: authenticated tests prove activate and deactivate return stream snapshot data | command_ref: CMD-BE-TEST.
- AC-AAS-02 | layer: backend | case: event conversion | red: unified agent events cannot be read as AG-UI-shaped DTOs | green: test activates a stream, invokes an agent with matching conversation code, and reads converted run/model/output events | command_ref: CMD-BE-TEST.
- AC-AAS-03 | layer: security | case: adapter redaction | red: converted events expose prompt, completion, raw tool output, API key, token, or request body | green: tests assert converted JSON excludes those values | command_ref: CMD-BE-TEST.
- AC-AAS-04 | layer: frontend | case: route wiring regression and browser smoke | red: route wiring breaks frontend build or authenticated app shell | green: `rtk make be-test`, `rtk make fe-check`, and in-app browser clicks through login/app shell without console-breaking errors | interaction: start backend and frontend -> open app -> log in through available auth path -> click navigation surfaces -> confirm app shell remains usable | command_ref: CMD-FE-CHECK.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red-green-verify | rerun: after adapter changes | expect: backend tests pass including AG-UI adapter tests.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after backend source changes | expect: backend compiles.
- CMD-FE-CHECK | command: `rtk make fe-check` | cost: medium | when: verify | rerun: after route wiring or before browser verification | expect: frontend typecheck and build pass.
- CMD-BROWSER-SMOKE | command: `in-app browser real-click smoke against local frontend/backend` | cost: slow/browser | when: verify | rerun: after frontend/backend route wiring changes or when evidence is stale | expect: app opens and real clicks do not expose broken shell behavior.
- CMD-DOCS-SPEC | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/agent-agui-stream-adapter --phase spec --complete-spec --no-downstream-artifacts` | cost: low | when: docs | rerun: after spec changes | expect: feature docs spec gate passes.

## Evidence Rules

- Browser evidence must list clicks, inputs, network, feedback, and artifacts.
- Adapter evidence must inspect converted JSON, not only event counts.

## Open Decisions

- None.
