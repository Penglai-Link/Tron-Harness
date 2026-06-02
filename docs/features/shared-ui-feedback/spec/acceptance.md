# Acceptance: Shared UI Feedback

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Acceptance Index

- AC-FEEDBACK-01 [must] [business] The application exposes a shared Toast provider and hook for feature pages through the shared UI boundary; proof: @ref:spec/test-plan.md#proof-rows.
- AC-FEEDBACK-02 [must] [business] User interaction feedback renders as a top-right floating toast using shared UI styles; proof: @ref:spec/test-plan.md#proof-rows.
- AC-FEEDBACK-03 [must] [business] Toast messages automatically disappear by default and support manual dismissal; proof: @ref:spec/test-plan.md#proof-rows.

## Acceptance Criteria

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-FEEDBACK-01 | The application exposes a shared Toast/Snackbar provider and hook for feature pages. | automated | TDD-FEEDBACK-01 RED proves the shared API is missing; GREEN passes typecheck/build after provider, hook, and exports exist. | must |
| AC-FEEDBACK-02 | User interaction feedback renders as a top-right floating toast using shared UI styles. | automated | TDD-FEEDBACK-02 RED proves a wired action has no top-right toast; GREEN Playwright evidence verifies placement and visible text. | must |
| AC-FEEDBACK-03 | Toast messages automatically disappear by default and support manual dismissal. | automated | TDD-FEEDBACK-03 RED proves no auto-dismiss/dismiss behavior; GREEN Playwright evidence verifies timeout disappearance and close behavior. | must |

## Business Acceptance

- [x] AC-FEEDBACK-01 Feature pages can call one shared feedback API instead of local one-off feedback state.
- [x] AC-FEEDBACK-02 Interaction feedback appears as a floating bubble at the top-right.
- [x] AC-FEEDBACK-03 Feedback automatically disappears without operator cleanup.

## UI Acceptance

- [x] AC-FEEDBACK-02 Toast viewport is fixed to the right side near the top of the viewport.
- [x] AC-FEEDBACK-02 Toast tones visually distinguish info, success, warning, and error using existing design tokens.
- [x] AC-FEEDBACK-03 Toasts expose a close control and auto-dismiss.

## Interaction Acceptance

- IA-FEEDBACK-01 | source: shared-shell action click | acceptance: AC-FEEDBACK-01, AC-FEEDBACK-02 | red: TDD-FEEDBACK-02 | status: verified
- IA-FEEDBACK-02 | source: toast viewport render | acceptance: AC-FEEDBACK-02 | red: TDD-FEEDBACK-02 | status: verified
- IA-FEEDBACK-03 | source: toast auto-dismiss timeout | acceptance: AC-FEEDBACK-03 | red: TDD-FEEDBACK-03 | status: verified
- IA-FEEDBACK-04 | source: toast close button | acceptance: AC-FEEDBACK-03 | red: TDD-FEEDBACK-03 | status: verified

## Backend Acceptance

Not applicable.

## Database Acceptance

Not applicable.

## Integration Acceptance

- [x] AC-FEEDBACK-01 `ToastProvider` is mounted at the app root.
- [x] AC-FEEDBACK-01 `useToast` is exported from the shared UI boundary.
- [x] AC-FEEDBACK-02 At least one existing page action uses the shared toast API as proof integration.

## Security And Privacy Acceptance

- [x] AC-FEEDBACK-01 The shared API does not log toast messages to backend or persistent storage.
- [x] AC-FEEDBACK-03 Toast messages are transient frontend state.

## Regression Acceptance

- [x] AC-FEEDBACK-01 `rtk make fe-check` passes.
- [x] AC-FEEDBACK-02 Browser runtime feedback test passes with concrete click, visible feedback, placement, and auto-dismiss evidence.

## Evidence Requirements

| evidence item | acceptance ids | TDD cycle ids | required evidence |
| --- | --- | --- | --- |
| Shared provider/hook implementation. | AC-FEEDBACK-01 | TDD-FEEDBACK-01 | RED type/source assertion fails before implementation; GREEN `rtk make fe-check` passes. |
| Shared action displays top-right toast. | AC-FEEDBACK-02 | TDD-FEEDBACK-02 | RED Playwright assertion fails before implementation; GREEN browser evidence records click and visible toast placement. |
| Auto-dismiss and manual dismissal. | AC-FEEDBACK-03 | TDD-FEEDBACK-03 | RED Playwright assertion fails before implementation; GREEN browser evidence records disappearance after timeout and close control behavior. |

## Open Decisions

- None.
