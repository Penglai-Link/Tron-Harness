# Test Plan: Shared UI Feedback

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-13

## Proof Rows

- AC-FEEDBACK-01 | layer: frontend | case: shared provider and hook API compile from the app and shared export boundary | red: `ToastProvider`, `useToast`, or public shared export is missing | green: typecheck and build pass after provider wiring and export updates | interaction: open the routed app shell and verify the shared provider-backed feedback API is available to page code | command_ref: CMD-FE-CHECK.
- AC-FEEDBACK-02 | layer: browser | case: a routed page action triggers a shared top-right toast | red: clicking the proof action does not create a visible shared toast | green: browser proof sees the shared toast viewport, visible message, and top-right placement under explicit mock auth/data mode | interaction: open `/shared-shell-preview` -> click `Export Scan` -> observe shared toast viewport and visible status feedback | command_ref: CMD-FEEDBACK-BROWSER.
- AC-FEEDBACK-03 | layer: browser | case: shared toast lifecycle supports auto-dismiss and manual dismiss | red: toast remains indefinitely or cannot be dismissed manually | green: browser proof sees timeout-based disappearance and close-control dismissal in explicit mock auth/data mode | interaction: trigger a toast -> wait for auto-dismiss -> trigger a second toast -> dismiss it with the close control | command_ref: CMD-FEEDBACK-BROWSER.

## Test Strategy

Use frontend-first TDD. RED introduces Playwright/source assertions proving there is no shared toast provider/hook and no top-right auto-dismiss feedback. GREEN adds the shared UI primitive, root provider mount, public export, and one existing shared-shell page action using the hook. VERIFY reruns type/build and the browser runtime test once. The browser proof runs in explicit mock auth/data mode so toast acceptance stays isolated from login-flow variance.

## TDD Cycle Index

### Positive Scenarios

- TDD-FEEDBACK-01 -> AC-FEEDBACK-01 | shared provider and hook API compile from the app/shared boundary | frontend | status: ready | focus: provider, hook, export
- TDD-FEEDBACK-02 -> AC-FEEDBACK-02 | shared-shell action click shows a top-right floating toast | browser | status: ready | focus: visible feedback placement

### Exception Scenarios

- TDD-FEEDBACK-03 -> AC-FEEDBACK-03 | toast disappears automatically and can be dismissed manually | browser | status: ready | focus: timeout and close behavior

### Boundary Scenarios

- TDD-FEEDBACK-04 -> AC-FEEDBACK-01 | feedback feature has no backend/database side effects | review | status: ready | focus: frontend-only boundary

## TDD Cycle Details

### TDD-FEEDBACK-01 Details

- Assertions: `ToastProvider` wraps the app root, `useToast` is exported from `@/shared`, and the API exposes `show`, tone helpers, and `dismiss`.
- RED proof: `rtk make fe-check` or Playwright source assertion fails because `Toast.tsx`, root provider wiring, and shared export are missing.
- GREEN/VERIFY evidence: `rtk make fe-check` passes after implementation.
- Status: ready

### TDD-FEEDBACK-02 Details

- Assertions: opening `/shared-shell-preview` in explicit mock auth/data mode and clicking `Export Scan` creates a visible toast with `role="status"` in a fixed top-right viewport.
- RED proof: `rtk make fe-test TEST='tests/shared-ui-feedback.spec.ts --grep "TDD-FEEDBACK-02"'` fails because no toast appears.
- GREEN/VERIFY evidence: the same command passes after shared feedback wiring.
- Status: ready

### TDD-FEEDBACK-03 Details

- Assertions: in the same explicit mock auth/data setup, a toast with a short duration disappears automatically, and another toast can be dismissed through its close control.
- RED proof: `rtk make fe-test TEST='tests/shared-ui-feedback.spec.ts --grep "TDD-FEEDBACK-03"'` fails because no auto-dismiss/close behavior exists.
- GREEN/VERIFY evidence: the same command passes after toast lifecycle implementation.
- Status: ready

### TDD-FEEDBACK-04 Details

- Assertions: implementation touches frontend shared UI/app/page/test paths only and does not introduce backend, database, or API files.
- RED proof: source review before implementation has no shared primitive.
- GREEN/VERIFY evidence: changed-path review confirms frontend-only scope.
- Status: ready

## Test Layers

- Frontend compile/typecheck/build: `rtk make fe-check`.
- Browser runtime interaction: focused `shared-ui-feedback.spec.ts` Playwright test.
- Backend/database layers: not applicable for this frontend-only feature.

## Unit Test Plan

Not applicable.

## Integration Test Plan

Type/build integration uses `rtk make fe-check`.

## Database Test Plan

Not applicable.

## Frontend Test Plan

`rtk make fe-check` validates TypeScript and production build for the shared UI API and app provider mount.

## Browser Acceptance Plan

Playwright opens `/shared-shell-preview` under explicit mock auth/data mode, clicks `Export Scan`, checks the toast viewport and status role, checks top-right placement, verifies auto-dismiss, triggers another toast, and dismisses it with the close button.

## Security And Privacy Test Plan

Review changed paths and implementation to ensure toast messages remain transient frontend state and no persistence/logging is added.

## Test Data And Fixtures

- Toast message: `Scan export requested`
- Toast title: `Export queued`
- Auto-dismiss duration: short duration configured by the proof integration for deterministic browser evidence.

## Regression Scope

- `rtk make fe-check`
- Focused `shared-ui-feedback.spec.ts` Playwright runtime test in explicit mock auth/data mode.
- Feature docs validation.

## Command Index

- CMD-DOCS-FINAL | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/shared-ui-feedback --phase final` | cost: fast | when: final | rerun: after spec or report edits | expect: implemented feature docs stay aligned with the current validator template.
- CMD-FE-CHECK | command: `rtk make fe-check` | cost: medium | when: RED baseline and GREEN/VERIFY | rerun: after frontend source changes | expect: frontend typecheck and production build pass.
- CMD-FEEDBACK-BROWSER | command: `rtk make fe-test TEST='tests/shared-ui-feedback.spec.ts'` | cost: slow/browser | when: verify | rerun: review-if-stale when feedback source or test files change or evidence is missing/failed | expect: browser runtime feedback evidence passes in explicit mock auth/data mode.

## Evidence Rules

- AC-FEEDBACK-01 evidence must prove the provider mount, shared export, and compile-time API surface through `CMD-FE-CHECK`.
- AC-FEEDBACK-02 evidence must record the routed click, visible toast text, `role="status"` feedback, and top-right placement from `CMD-FEEDBACK-BROWSER`.
- AC-FEEDBACK-03 evidence must record both auto-dismiss and manual-dismiss behavior from the same browser proof run.
- Feature docs validation for this implemented slice uses `CMD-DOCS-FINAL` rather than the pre-implementation spec gate.
