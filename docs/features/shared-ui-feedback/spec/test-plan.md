# Test Plan: Shared UI Feedback

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-13

## Test Strategy

Use frontend-first TDD. RED introduces Playwright/source assertions proving there is no shared toast provider/hook and no top-right auto-dismiss feedback. GREEN adds the shared UI primitive, root provider mount, public export, and one existing shared-shell page action using the hook. VERIFY reruns type/build and the browser runtime test once.

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

- Assertions: clicking `Export Scan` on `/shared-shell-preview` creates a visible toast with `role="status"` in a fixed top-right viewport.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test shared-ui-feedback.spec.ts --grep "TDD-FEEDBACK-02" --reporter=line` fails because no toast appears.
- GREEN/VERIFY evidence: the same command passes after shared feedback wiring.
- Status: ready

### TDD-FEEDBACK-03 Details

- Assertions: a toast with a short duration disappears automatically, and another toast can be dismissed through its close control.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test shared-ui-feedback.spec.ts --grep "TDD-FEEDBACK-03" --reporter=line` fails because no auto-dismiss/close behavior exists.
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

Playwright opens `/shared-shell-preview`, clicks `Export Scan`, checks the toast viewport and status role, checks top-right placement, verifies auto-dismiss, triggers another toast, and dismisses it with the close button.

## Security And Privacy Test Plan

Review changed paths and implementation to ensure toast messages remain transient frontend state and no persistence/logging is added.

## Test Data And Fixtures

- Toast message: `Scan export requested`
- Toast title: `Export queued`
- Auto-dismiss duration: short duration configured by the proof integration for deterministic browser evidence.

## Regression Scope

- `rtk make fe-check`
- Focused `shared-ui-feedback.spec.ts` Playwright runtime test.
- Feature docs validation.

## Command Index

- `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/shared-ui-feedback --phase spec --complete-spec --no-downstream-artifacts` | purpose: validate complete spec before tasks | cost: fast | when: spec gate | rerun: after spec edits
- `rtk make fe-check` | purpose: frontend typecheck and production build | cost: medium | when: RED baseline and GREEN/VERIFY | rerun: after frontend source changes
- `rtk npm --prefix tron-frontend exec playwright -- test shared-ui-feedback.spec.ts --reporter=line` | purpose: browser runtime feedback evidence | cost: slow/browser | when: verify | rerun: review-if-stale when feedback source/test files change or evidence is missing/failed
