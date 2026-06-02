# Feature Spec: Shared UI Feedback

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

Repository scan found no unified `Toast` or `Snackbar` API, no `sonner`, `notistack`, `react-hot-toast`, or toastify dependency, and only local page-level `setNotice` state. This feature creates the shared frontend feedback utility required before the paused user-management loading/feedback evolution continues.

## Feature Identity And Context

- feature_slug: shared-ui-feedback
- feature_name: Shared UI Feedback
- status: implemented
- owner: clouds3n
- source_request: Create a new feature to封装统一的提示工具 because the project has no unified Toast/Snackbar wrapper.
- main_agent: Codex lead agent
- dependency_features: none
- dependency_contracts: frontend app root provider boundary, shared UI export boundary, shared-shell preview route, current design-token system

## Requirement

Provide a shared frontend Toast/Snackbar feedback utility for all user interaction feedback. Feedback messages must be rendered through the shared UI layer, float at the top-right of the viewport, and automatically disappear after a configured duration.

The utility must expose a reusable hook and provider so feature pages can show success, info, warning, or error feedback without maintaining local ad hoc feedback elements. The initial integration proves the provider works from the app root and that an existing page action can trigger the shared feedback surface.

## Product Source Slices

- selected_feature: `Feature: Shared UI Feedback`
- selected_page: `Page: Application Root`; `Page: /shared-shell-preview`
- selected_interactions: `Interaction: Trigger Shared Toast`; `Interaction: Auto Dismiss Toast`; `Interaction: Manually Dismiss Toast`
- selected_rules: `Rule: Shared Feedback API`; `Rule: Top Right Toast Placement`; `Rule: Transient Frontend Only Feedback`
- selected_exceptions: `Exception: No Unified Toast Primitive Exists`
- selected_acceptance_source: `Acceptance: Shared UI Feedback`
- selected_workspace: `docs/features/shared-ui-feedback`
- feature_inventory_summary: this feature adds the shared feedback primitive that later page-level interaction work can reuse
- shared_requirement_summary: the app root owns provider mounting and shared-shell preview provides the first runtime proof action
- dependency_map_summary: this slice has no upstream feature dependency and becomes a downstream dependency for features that need shared operator feedback
- implementation_order_summary: add the shared primitive and app-root mount first, then prove it through one routed page action and focused browser coverage

## Scope Boundary

This feature owns one frontend-only shared UI primitive plus the minimal proof integration needed to show it working from the app root.

## Assumptions

- Tron currently uses a custom shared UI layer rather than a third-party component framework.
- The shared wrapper is the project-level UI framework primitive for feedback until a broader design-system package exists.
- Future feature work, including user-management, must call this wrapper for interaction feedback.

## In Scope

- Add a shared `ToastProvider` mounted at the application root.
- Add a reusable `useToast` hook with `show`, `success`, `error`, `warning`, `info`, and `dismiss` operations.
- Render toast bubbles in the top-right corner with accessible live-region semantics.
- Auto-dismiss toast messages by default while allowing explicit duration override.
- Support manual dismissal for long enough messages.
- Keep the visual style consistent with the current custom shared UI system and design tokens.
- Prove a visible page action uses the shared utility in browser runtime evidence.

## Out Of Scope

- Refactor every existing page-level `setNotice` interaction in this slice.
- Change user-management Export, Save, or mutation loading behavior in this slice.
- Add a backend API, database table, or external notification service.
- Add a new third-party toast dependency unless the existing stack already requires one.

## Constraints

- Console operators and admins see feedback for their own frontend interactions.
- No new permission model is introduced.
- Toast visibility is a frontend UI concern and does not expose sensitive backend data.
- No backend, database, API route, or generated client boundary changes are in scope.

## Feature Model

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

## Page And Module Surface

- Application root:
  - entry point: `tron-frontend/src/app/App.tsx`
  - behavior: wraps all routed pages with `ToastProvider`
- Shared shell preview:
  - route: `/shared-shell-preview`
  - behavior: an existing action triggers the shared toast utility so browser evidence can verify placement and auto-dismiss behavior

- `tron-frontend/src/shared/ui/Toast.tsx`: shared provider, viewport, hook, and toast API.
- `tron-frontend/src/shared/index.ts`: public export for feature pages.
- `tron-frontend/src/app/App.tsx`: root provider mount.
- `tron-frontend/src/domain/shared-shell/pages/SharedShellPreviewPage.tsx`: proof integration for runtime evidence only.
- `tron-frontend/tests/shared-ui-feedback.spec.ts`: browser runtime acceptance evidence.

## Repository Findings

- `tron-frontend/package.json` has React, Radix Slot, class-variance-authority, lucide icons, Tailwind, and Zustand, but no toast/snackbar dependency.
- `tron-frontend/src/shared/ui` contains shared primitives such as `Button`, `DataTable`, `Forms`, `TablePagination`, and `Tabs`, but no feedback surface.
- Existing pages use local `setNotice` state for feedback, which is not a unified top-right auto-dismiss Toast/Snackbar.
- `tron-frontend/tests/user-management-smoke.spec.ts` already uses Playwright runtime tests with a local Vite server pattern.

## Open Decisions

- None.
