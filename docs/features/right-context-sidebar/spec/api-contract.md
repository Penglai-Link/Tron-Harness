# API Contract: Right Context Sidebar

## API Overview

- status: approved
- decision: no backend API contract changes

The feature is a frontend shell and page composition change. Existing page APIs continue to provide the same data to pages. Moving contextual inspectors into the shell-level right sidebar does not change request or response bodies.

## Endpoints

No endpoints are added, changed, or removed.

## Request And Response DTOs

No DTOs change.

## Auth And Permissions

No auth or permission model changes.

The shell remains mounted only inside existing protected routes. Settings routes continue to use `SettingsShell` and remain outside this feature.

## Error Contract

No API error codes change.

Frontend UI should preserve existing page-level error and empty states after inspector migration.

## Frontend Backend Boundary

Frontend owns:

- right sidebar collapsed state,
- mobile drawer open state,
- shell rendering and accessibility controls,
- migration of existing contextual inspector presentation into `useConsoleShellConfig({ rightPanel })`.

Backend owns:

- unchanged domain data returned to current pages.

## Database And Transaction Boundary

No database or transaction boundary changes.

## Mock Or Adapter Expectations

Existing mock data and Playwright route handlers remain valid. Tests should prove no new API dependency is required for shell sidebar behavior.

## Open Decisions

- None.
