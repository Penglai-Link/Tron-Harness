# Test Plan: App Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user
- approved_at: 2026-05-19

## Proof Rows

- AC-APP-01 | layer: browser | case: `/apps` reflects current workspace, filters, and no import actions | red: placeholder app page exposed import buttons and non-workspace fields | green: app-management Playwright smoke proves workspace scoping, filters, and missing import button | interaction: preload the persisted shell selection as `TRON_CORE` -> open `/apps` -> switch status/runtime filters -> observe only workspace rows and no import action -> network `GET /api/v1/apps/workspaces/TRON_CORE*` | command_ref: CMD-PW-APP.
- AC-APP-02 | layer: browser | case: create app through `/apps/new` | red: no create page or persisted create flow existed | green: smoke creates `Playwright Runtime` and lands on detail | interaction: click `Create App` -> fill form -> save -> detail page visible -> network `POST /api/v1/apps/workspaces/TRON_CORE` | command_ref: CMD-PW-APP.
- AC-APP-03 | layer: browser | case: detail + edit pages update app metadata | red: no detail/edit routes existed | green: smoke edits name/runtime role/branch/tags and detail reflects updates | interaction: open detail -> edit -> save -> detail shows updated values -> network `PATCH /api/v1/apps/{appCode}` | command_ref: CMD-PW-APP.
- AC-APP-04 | layer: api | case: archive/restore removes and re-adds task options | red: no app lifecycle endpoints or app option registry existed | green: backend integration tests prove archive/restore and options exclusion/reinclusion | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-APP-04 | layer: browser | case: archived app row can be found and restored after archive | red: placeholder page had no lifecycle | green: smoke archives an app, verifies archived filter, restores it, and returns to active list | interaction: archive in detail -> view archived list -> restore -> list returns active row | command_ref: CMD-PW-APP.
- AC-APP-05 | layer: api | case: app list/create/update/lifecycle contracts | red: routes absent before app domain implementation | green: backend tests prove list filters, duplicate-name conflict, create/update, and lifecycle endpoints | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-APP-06 | layer: api | case: task write rejects unknown, archived, and foreign-workspace app codes | red: task writes accepted free-text app codes | green: backend tests return `TASK_APP_INVALID` for invalid app selections | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-APP-06 | layer: browser | case: task pages select active apps and preserve legacy codes until explicit change | red: task create/detail only exposed free-text `App Codes` and always resent them | green: task Playwright smoke uses app options, proves app updates travel through requests, and legacy codes remain visible without unrelated resubmission | interaction: create/detail task -> select app buttons -> save unrelated change with legacy code retained -> remove legacy code explicitly -> network appCodes only when selection changes | command_ref: CMD-PW-TASK.
- AC-APP-01 | layer: frontend | case: TypeScript build remains healthy after route/model migration | red: new pages/hooks could break frontend compilation | green: `make fe-check` passes | interaction: not applicable | command_ref: CMD-FE-CHECK.

## Command Index

- CMD-BE-TEST | command: make be-test | cost: medium | when: green/verify | rerun: affected backend or API validation changes | expect: backend app + task integration tests pass.
- CMD-FE-CHECK | command: make fe-check | cost: medium | when: green/verify | rerun: affected frontend route, model, or component changes | expect: TypeScript check and production build pass.
- CMD-PW-APP | command: rtk make fe-test TEST='tests/app-management-smoke.spec.ts' | cost: slow/browser | when: verify | rerun: review-if-stale | expect: real browser flow for app list/create/detail/edit/archive/restore.
- CMD-PW-TASK | command: rtk make fe-test TEST='tests/task-management-smoke.spec.ts' | cost: slow/browser | when: verify | rerun: review-if-stale | expect: task selector migration, legacy app behavior, and workspace-scoped app requests.
- CMD-DOC-VALIDATE | command: python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/app-management --phase final | cost: fast | when: final | rerun: after doc changes | expect: AC workspace passes validator.

## Evidence Rules

- Every `AC-APP-*` row above is covered by at least one API, frontend, or browser proof row.
- Browser evidence must keep concrete clicks, inputs, network assertions, and visible feedback in reports.
- Review reuses fresh Playwright evidence from the final verify run unless implementation changes afterward.
