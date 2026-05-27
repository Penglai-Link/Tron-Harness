# Test Plan: Agent Run History

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user delegated decisions to Codex
- approved_at: 2026-05-27

## Proof Rows

- AC-ARH-01 | layer: backend | case: successful invocation writes run history | red: invoking an agent returns output but no persisted run summary exists | green: API test invokes a run and then reads a persisted summary with identity, completed status, step count, observation count, timing, and bounded output preview | command_ref: CMD-BE-TEST.
- AC-ARH-02 | layer: api | case: list and detail history APIs | red: run history has no protected query endpoint or paged shape | green: authenticated tests prove list paging and detail retrieval use uniform envelopes | command_ref: CMD-BE-TEST.
- AC-ARH-03 | layer: security | case: history masking | red: prompt, raw completion, tool output, API key, token, or request body appears in history JSON | green: tests assert history JSON excludes those values and includes only previews/diagnostics | command_ref: CMD-BE-TEST.
- AC-ARH-04 | layer: api | case: missing run lookup | red: missing detail fails with an unstable error | green: test proves `GET /api/v1/agents/runs/MISSING` returns `AGENT_RUN_NOT_FOUND` | command_ref: CMD-BE-TEST.
- AC-ARH-05 | layer: regression | case: previous feature compatibility | red: history wiring breaks runtime or invocation tests | green: `rtk make be-test` and `rtk make be-check` pass with feature1/2 tests included | command_ref: CMD-BE-TEST.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red-green-verify | rerun: after backend history changes | expect: backend tests pass including agent history tests.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after backend source changes | expect: backend compiles.
- CMD-DOCS-SPEC | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/agent-run-history --phase spec --complete-spec --no-downstream-artifacts` | cost: low | when: docs | rerun: after spec changes | expect: feature docs spec gate passes.

## Evidence Rules

- RED evidence must show missing persistence or missing API before GREEN.
- Security evidence must inspect response JSON.
- Browser evidence for this backend feature can reuse final app-shell regression evidence from feature4 when both features ship together.

## Open Decisions

- None.
