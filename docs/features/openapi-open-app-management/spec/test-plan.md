# Test Plan: OpenAPI Open-App Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user approved the product direction and delegated bounded feature implementation details to Codex
- approved_at: 2026-06-11

## Proof Rows

- AC-OAM-01 | layer: backend | case: list-create-detail-update-rotate lifecycle | red: there is no platform admin API to manage open-app lifecycle or key rotation, and cleartext secrets cannot be revealed in a controlled one-time response | green: backend tests prove create returns a reveal payload, list and detail omit cleartext keys, update changes name or status, and rotate returns a new reveal payload while revoking the old key | command_ref: CMD-BE-TEST.
- AC-OAM-02 | layer: backend | case: shared capability catalog output | red: runtime route authorization and operator capability catalog are duplicated or inconsistent | green: backend tests prove the admin catalog endpoint and runtime auth registry expose the same grouped capability-operation set for current agent-chat openapi routes | command_ref: CMD-BE-TEST.
- AC-OAM-03 | layer: backend | case: grant replacement and validation | red: admin flows cannot replace grant sets atomically, invalid users pass silently, or invalid capability pairs are accepted | green: backend tests prove explicit user grants and capability-operation grants are replaced per app, invalid user codes fail, and invalid capability-operation pairs fail with the standard envelope | command_ref: CMD-BE-TEST.
- AC-OAM-04 | layer: frontend | case: settings page list and edit flow | red: `/settings/openapi` does not exist or cannot show open-apps, catalog groups, current grants, and one-time secret reveal states | green: frontend build and browser checks prove the settings page loads in the settings shell, creates an open-app, shows the one-time key reveal, and saves explicit user plus capability grants | interaction: open Settings -> open OpenAPI -> create open-app -> inspect reveal panel -> edit user grants -> save capability grants -> verify detail refresh | command_ref: CMD-BROWSER-OPENAPI.
- AC-OAM-05 | layer: browser | case: naming and regression guard | red: the page leaks app-management naming, hides capability grouping, or assumes legacy raw-key management | green: browser evidence proves the page consistently uses `open-app` wording, groups current openapi capabilities, and presents rotate-only secret lifecycle after first reveal | interaction: open Settings -> open OpenAPI -> inspect page copy -> create open-app -> dismiss reveal -> rotate key -> verify new reveal appears and list or detail still shows fingerprint only | command_ref: CMD-BROWSER-OPENAPI.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red-green-verify | rerun: after backend service, repository, or admin route changes | expect: backend tests pass including open-app management lifecycle and grant coverage.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after backend module or route wiring changes | expect: backend compiles cleanly.
- CMD-FE-CHECK | command: `rtk make fe-check` | cost: medium | when: red-green-verify | rerun: after frontend page, route, or API client changes | expect: frontend compiles and type-checks cleanly.
- CMD-DOCS-WORKTREE | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/openapi-open-app-management --phase worktrees` | cost: low | when: docs | rerun: after spec, tasks, or worktree-plan changes | expect: feature docs pass the worktree-phase validator.
- CMD-BROWSER-OPENAPI | command: `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts` | cost: slow/browser | when: final-verify | rerun: after frontend interaction or admin API contract changes | expect: real browser flow logs in as the bootstrap platform admin, creates an open-app, saves user and capability grants, rotates the key, and refreshes screenshot plus network artifacts under `docs/features/openapi-open-app-management/reports/`.

## Evidence Rules

- Secret-lifecycle evidence must confirm cleartext `appKey` appears only immediately after create or rotate.
- Catalog evidence must compare operator-facing grouped operations with backend route-auth registry output.
- Grant evidence must distinguish empty grant state, replacement success, and invalid-target rejection.
- Browser evidence must name clicks, inputs, visible state changes, and any network assertions used.

## Open Decisions

- None.
