# API Contract: Shell Theme Switcher

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## API Overview

No backend HTTP API is introduced. The public contract is a frontend runtime theme API.

## Endpoints

Not applicable.

## Request And Response DTOs

Frontend API:

- `ThemeProvider({ children })`: mounts theme state for the app.
- `useTheme()`: returns `{ theme, themes, setTheme }`.
- `AppTheme`: `light` or `matrix-dark`.
- `ThemeOption`: selector option metadata for the user menu.

Persistence contract:

- Storage key: `tron.theme`
- Storage owner: browser local storage
- Storage values: `light`, `matrix-dark`

## Auth And Permissions

No new permission model is introduced. The selector is visible inside the authenticated user settings menu because that is where the user requested the control to live.

## Error Contract

If `useTheme` is called outside `ThemeProvider`, the hook throws a developer-facing error. Runtime theme switching does not produce a backend error envelope because no HTTP request is involved.

## Frontend Backend Boundary

Theme changes are strictly client-side. Existing auth and workspace-switcher requests continue to run as before, but the theme selector itself does not call a server route.

## Database And Transaction Boundary

Not applicable.

## Mock Or Adapter Expectations

Browser proof may run against mocked auth and workspace-switcher routes for focused Playwright verification, while live Chrome verification may run against the existing local backend. Neither path requires a theme backend adapter.

## Open Decisions

- None.
