# Acceptance: Shared UI Feedback

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Acceptance Criteria

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-FEEDBACK-01 | The application exposes a shared Toast/Snackbar provider and hook for feature pages. | automated | TDD-FEEDBACK-01 RED proves the shared API is missing; GREEN passes typecheck/build after provider, hook, and exports exist. | must |
| AC-FEEDBACK-02 | User interaction feedback renders as a top-right floating toast using shared UI styles. | automated | TDD-FEEDBACK-02 RED proves a wired action has no top-right toast; GREEN Playwright evidence verifies placement and visible text. | must |
| AC-FEEDBACK-03 | Toast messages automatically disappear by default and support manual dismissal. | automated | TDD-FEEDBACK-03 RED proves no auto-dismiss/dismiss behavior; GREEN Playwright evidence verifies timeout disappearance and close behavior. | must |

## Business Acceptance

- [ ] AC-FEEDBACK-01 Feature pages can call one shared feedback API instead of local one-off feedback state.
- [ ] AC-FEEDBACK-02 Interaction feedback appears as a floating bubble at the top-right.
- [ ] AC-FEEDBACK-03 Feedback automatically disappears without operator cleanup.

## UI Acceptance

- [ ] AC-FEEDBACK-02 Toast viewport is fixed to the right side near the top of the viewport.
- [ ] AC-FEEDBACK-02 Toast tones visually distinguish info, success, warning, and error using existing design tokens.
- [ ] AC-FEEDBACK-03 Toasts expose a close control and auto-dismiss.

## Interaction Acceptance Checklist

- IA-FEEDBACK-01 | source: shared-shell action click | acceptance: AC-FEEDBACK-01, AC-FEEDBACK-02 | red: TDD-FEEDBACK-02 | status: ready
- IA-FEEDBACK-02 | source: toast viewport render | acceptance: AC-FEEDBACK-02 | red: TDD-FEEDBACK-02 | status: ready
- IA-FEEDBACK-03 | source: toast auto-dismiss timeout | acceptance: AC-FEEDBACK-03 | red: TDD-FEEDBACK-03 | status: ready
- IA-FEEDBACK-04 | source: toast close button | acceptance: AC-FEEDBACK-03 | red: TDD-FEEDBACK-03 | status: ready

## Backend Acceptance

Not applicable.

## Database Acceptance

Not applicable.

## Integration Acceptance

- [ ] AC-FEEDBACK-01 `ToastProvider` is mounted at the app root.
- [ ] AC-FEEDBACK-01 `useToast` is exported from the shared UI boundary.
- [ ] AC-FEEDBACK-02 At least one existing page action uses the shared toast API as proof integration.

## Security And Privacy Acceptance

- [ ] AC-FEEDBACK-01 The shared API does not log toast messages to backend or persistent storage.
- [ ] AC-FEEDBACK-03 Toast messages are transient frontend state.

## Regression Acceptance

- [ ] AC-FEEDBACK-01 `rtk make fe-check` passes.
- [ ] AC-FEEDBACK-02 Browser runtime feedback test passes with concrete click, visible feedback, placement, and auto-dismiss evidence.

## Evidence Requirements

| evidence item | acceptance ids | TDD cycle ids | required evidence |
| --- | --- | --- | --- |
| Shared provider/hook implementation. | AC-FEEDBACK-01 | TDD-FEEDBACK-01 | RED type/source assertion fails before implementation; GREEN `rtk make fe-check` passes. |
| Shared action displays top-right toast. | AC-FEEDBACK-02 | TDD-FEEDBACK-02 | RED Playwright assertion fails before implementation; GREEN browser evidence records click and visible toast placement. |
| Auto-dismiss and manual dismissal. | AC-FEEDBACK-03 | TDD-FEEDBACK-03 | RED Playwright assertion fails before implementation; GREEN browser evidence records disappearance after timeout and close control behavior. |
