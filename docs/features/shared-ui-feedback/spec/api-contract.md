# API Contract: Shared UI Feedback

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## API Overview

No backend HTTP API is introduced. The public contract is a frontend shared UI API.

## Endpoints

Not applicable.

## Request And Response DTOs

Frontend API:

- `ToastProvider({ children })`: mounts the shared feedback viewport.
- `useToast()`: returns the toast API.
- `toast.show({ tone, title, message, durationMs })`: creates a toast and returns its id.
- `toast.success(message, options)`, `toast.info(message, options)`, `toast.warning(message, options)`, `toast.error(message, options)`: convenience operations.
- `toast.dismiss(id)`: removes a toast.

Toast fields:

- `id`: generated client id.
- `tone`: `info`, `success`, `warning`, or `error`.
- `title`: optional short title.
- `message`: required visible message.
- `durationMs`: optional auto-dismiss timeout; defaults to the provider default.

## Auth And Permissions

Not applicable. Toast display is local UI behavior.

## Error Contract

If `useToast` is called outside `ToastProvider`, the hook throws a developer-facing error. Runtime pages must mount `ToastProvider` at the app root.

## Frontend Backend Boundary

Frontend-only. Backend responses may later be mapped to toast messages by feature pages, but this feature does not change backend envelopes.

## Database And Transaction Boundary

Not applicable.

## Mock Or Adapter Expectations

Browser tests may trigger a shared-shell action that uses `useToast`; no network mock is needed.

## Open Decisions

- None.
