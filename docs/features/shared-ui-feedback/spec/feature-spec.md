# Feature Spec: Shared UI Feedback

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

Repository scan found no unified `Toast` or `Snackbar` API, no `sonner`, `notistack`, `react-hot-toast`, or toastify dependency, and only local page-level `setNotice` state. This feature creates the shared frontend feedback utility required before the paused user-management loading/feedback evolution continues.

## Feature Identity

- feature_slug: shared-ui-feedback
- status: approved_with_defaults
- owner: clouds3n
- source_request: Create a new feature to封装统一的提示工具 because the project has no unified Toast/Snackbar wrapper.
- main_agent: Codex lead agent

## Requirement Description

Provide a shared frontend Toast/Snackbar feedback utility for all user interaction feedback. Feedback messages must be rendered through the shared UI layer, float at the top-right of the viewport, and automatically disappear after a configured duration.

The utility must expose a reusable hook and provider so feature pages can show success, info, warning, or error feedback without maintaining local ad hoc feedback elements. The initial integration proves the provider works from the app root and that an existing page action can trigger the shared feedback surface.

## Goals And Non-Goals

Goals:

- Add a shared `ToastProvider` mounted at the application root.
- Add a reusable `useToast` hook with `show`, `success`, `error`, `warning`, `info`, and `dismiss` operations.
- Render Toast/Snackbar bubbles in the top-right corner with accessible live-region semantics.
- Auto-dismiss toast messages by default while allowing explicit duration override.
- Support manual dismissal for long enough messages.
- Keep the visual style consistent with the current custom shared UI system and design tokens.
- Prove a visible page action uses the shared utility in browser runtime evidence.

Non-goals:

- Refactor every existing page-level `setNotice` interaction in this feature.
- Change user-management Export, Save, or mutation loading behavior in this feature.
- Add a backend API, database table, or external notification service.
- Add a new third-party toast dependency unless the existing stack already requires one.

Assumptions:

- Tron currently uses a custom shared UI layer rather than a third-party component framework.
- The shared wrapper is the project-level UI framework primitive for feedback until a broader design-system package exists.
- Future feature work, including user-management, must call this wrapper for interaction feedback.

## User Roles And Permissions

- Console operators and admins see feedback for their own frontend interactions.
- No new permission model is introduced.
- Toast visibility is a frontend UI concern and does not expose sensitive backend data.

## Page Descriptions

Application root:

- Entry point: `tron-frontend/src/app/App.tsx`
- Behavior: wraps all routed pages with `ToastProvider`.

Shared shell preview:

- Route: `/shared-shell-preview`
- Behavior: at least one existing action triggers the shared toast utility so browser evidence can verify placement and auto-dismiss behavior.

## Interactions

- IA-FEEDBACK-01: Clicking a wired shared-shell action creates a toast through `useToast`.
- IA-FEEDBACK-02: A rendered toast appears in the top-right floating viewport.
- IA-FEEDBACK-03: A rendered toast automatically disappears after its configured duration.
- IA-FEEDBACK-04: A rendered toast can be manually dismissed with a close control.

## Domain Model

Bounded context:

- Shared Frontend UI Feedback.

Entities:

- `ToastMessage`: client-only UI message with id, tone, optional title, message text, and duration.

Value objects:

- `ToastTone`: `info`, `success`, `warning`, or `error`.
- `ToastDuration`: milliseconds before auto-dismiss.

Policies:

- Feedback for user interactions must use the shared toast API rather than one-off inline feedback state.
- Default placement is top-right.
- Default duration auto-dismisses messages without requiring user action.

## Module And Boundary Map

- `tron-frontend/src/shared/ui/Toast.tsx`: shared provider, viewport, hook, and toast API.
- `tron-frontend/src/shared/index.ts`: public export for feature pages.
- `tron-frontend/src/app/App.tsx`: root provider mount.
- `tron-frontend/src/domain/shared-shell/pages/SharedShellPreviewPage.tsx`: proof integration for runtime evidence only.
- `tron-frontend/tests/shared-ui-feedback.spec.ts`: browser runtime acceptance evidence.

No backend, database, API route, or generated client boundary changes are in scope.

## Repository Findings

- `tron-frontend/package.json` has React, Radix Slot, class-variance-authority, lucide icons, Tailwind, and Zustand, but no toast/snackbar dependency.
- `tron-frontend/src/shared/ui` contains shared primitives such as `Button`, `DataTable`, `Forms`, `TablePagination`, and `Tabs`, but no feedback surface.
- Existing pages use local `setNotice` state for feedback, which is not a unified top-right auto-dismiss Toast/Snackbar.
- `tron-frontend/tests/user-management-smoke.spec.ts` already uses Playwright runtime tests with a local Vite server pattern.

## Feature Dependencies

Strong feature dependencies: none found after scanning sibling `docs/features/*/spec/feature-spec.md`. This feature creates a shared capability that later user-management work depends on, but it does not require another feature to be designed or implemented first.

## Open Decisions

- None.
