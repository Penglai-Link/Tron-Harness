# Test Plan: Agent Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-29

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-AGENT-01 | layer: backend | case: platform `Main Agent` config persists and is consumed by main-agent runtime preparation | red: the config can be saved but runtime requests still omit the saved guidance, or runtime uses hard-coded instructions instead of persisted values | green: backend tests prove get and patch endpoints persist the singleton config, bootstrap fallback remains stable when missing, and main-agent invocation paths include the saved guidance in the model request while non-main-agent paths do not regress | interaction: save `personality` and `instructions` in `/settings/agents` -> trigger a main-agent style request -> verify the model request includes the saved guidance | command_ref: CMD-AGENT-BE-TEST.
- AC-AGENT-02 | layer: backend | case: system-agent registry scan preserves rows, platform policy, and `Unavailable` semantics | red: startup or manual scan fails to create known rows, a missing executable deletes the row, or rescan resets global enable or disable policy | green: backend tests prove known supported agents are scanned, global enable or disable persists across rescans, missing agents become `Unavailable`, and unavailable rows reject enable or disable actions until they reappear | interaction: bootstrap with supported probe results -> disable one system agent -> rescan with that agent missing -> verify row remains known and `Unavailable` -> rescan with agent available again -> verify policy survives | command_ref: CMD-AGENT-BE-TEST.
- AC-AGENT-03 | layer: backend | case: workspace-effective system-agent rows follow platform baseline plus current-workspace local disable overrides | red: workspace list includes `Main Agent`, a workspace enable can resurrect a platform-disabled agent, or local disable mutates platform policy | green: backend tests prove workspace rows list only system agents, `Inherited From` resolves to `Platform`, current-workspace disable persists locally, enable removes the local override, and platform disable dominates every workspace | interaction: list workspace rows -> disable one platform-enabled row locally -> verify workspace row becomes `Disabled` while platform remains `Enabled` -> re-enable -> verify inherited behavior returns; disable the same row globally -> verify workspace cannot make it effectively enabled | command_ref: CMD-AGENT-BE-TEST.
- AC-AGENT-04 | layer: browser | case: platform and workspace pages render the agreed controls and feedback | red: `/settings/agents` lacks `Scan`, workspace `/agents` shows `Main Agent`, system rows expose archive, or action buttons do not match the row state | green: browser smoke proves `/settings/agents` shows the main-agent editor and system-agent table with `Scan`, `/agents` excludes `Main Agent`, status and action labels match enabled or disabled or unavailable state, and success or failure feedback is visible on save or toggle flows | interaction: open `/settings/agents` -> edit and save `Main Agent` fields -> click `Scan` -> toggle a platform system agent -> open `/agents` -> verify inherited rows only -> disable then re-enable one workspace row -> confirm feedback and status transitions | command_ref: CMD-AGENT-BROWSER.

## Command Index

- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: backend agent-management code compiles cleanly.
- CMD-AGENT-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify/final | rerun: affected backend, persistence, or runtime changes | expect: agent-management backend tests pass along with existing backend test suite.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: platform and workspace agent-management pages compile and build.
- CMD-AGENT-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test agent-management-smoke.spec.ts --reporter=line | cost: slow/browser | when: verify/final | rerun: affected frontend page, mock, or API wiring changes | expect: `/settings/agents` and `/agents` match the accepted interactions.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/agent-management --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: agent-management feature docs validate end to end.

## Evidence Rules

- Every `AC-AGENT-*` row must have final evidence in reports.
- Backend evidence for AC-AGENT-01 must mention both persistence and runtime consumption.
- Backend evidence for AC-AGENT-02 must mention `Unavailable` and policy preservation across rescan.
- Backend and browser evidence for AC-AGENT-03 must explicitly confirm that `Main Agent` is absent from `/agents`.
- Browser evidence for AC-AGENT-04 must list clicks, visible feedback, and any relevant network assertions.
