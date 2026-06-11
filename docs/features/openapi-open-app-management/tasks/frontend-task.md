# Frontend Task: OpenAPI Open-App Management

## Execution Contract

- feature_slug: openapi-open-app-management
- status: done
- execution_mode: solo

## TDD Contract

Frontend work stays inside the settings shell, uses `open-app` terminology consistently, and never persists or re-fetches cleartext secrets after the initial reveal response.

## Task Rows

- AC-OAM-04 | phase: tdd | owner: lead | scope: `/settings/openapi` route registration, settings navigation entry, frontend API client, and platform open-app management page | command_ref: CMD-BROWSER-OPENAPI | status: done.
- AC-OAM-05 | phase: tdd | owner: lead | scope: one-time key reveal UX, grouped capability display, and explicit user plus capability grant editing flow | command_ref: CMD-BROWSER-OPENAPI | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/openapi-open-app-management/base`.
- Branch: `feature/clouds3n/openapi-open-app-management/base`.

## Integration Notes

- The page should remain platform-level only.
- Avoid reuse of the existing app-management route names or labels.

## Open Decisions

- None.
