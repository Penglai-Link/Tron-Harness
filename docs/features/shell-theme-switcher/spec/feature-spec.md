# Feature Spec: Shell Theme Switcher

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: shell-theme-switcher
- feature_name: Shell Theme Switcher
- status: approved_with_defaults
- owner: clouds3n
- source_request: 给右上角用户设置里加主题选择器，保留当前浅色主题，并新增一套可实时切换的暗色 Matrix 配色方案。
- main_agent: Codex lead agent
- dependency_features: `auth`, `workspace-management`, `shared-ui-feedback`
- dependency_contracts: existing protected `TronShell` user menu; existing auth session bootstrap; shared frontend CSS-token surfaces and toast infrastructure

## Requirement

Add a theme selector to the authenticated top-right user settings panel. Keep the current light theme as the default built-in theme, add a dark `Matrix Dark` palette, and let operators switch themes in real time.

The selected theme must affect the mounted shell immediately, remain available inside the existing user settings flow, and persist across reloads without introducing a backend dependency.

## Scope Boundary

This feature is frontend-only. It changes the app root theme runtime, the shell user-menu surface, the global design-token layer, and shared elevated UI surfaces that would otherwise stay hardcoded light.

## Assumptions

- Tron already mounts the authenticated shell around protected routes and already has a right-top user menu surface that can host additional settings.
- Persisting the theme in browser-local state is sufficient for the current operator experience; no cross-device sync is required.
- Shared primitives such as shell panels, cards, tabs, tables, and toast surfaces should participate in dark-mode tokens so the Matrix theme does not leave large white blocks behind.

## In Scope

- Add a frontend `ThemeProvider` and theme hook at the app root.
- Persist the selected theme in browser local storage.
- Add a `Theme` selector to the existing user settings panel ahead of the language selector.
- Introduce a `Matrix Dark` token set and apply it to global CSS custom properties.
- Replace key shared hardcoded light surfaces with theme-aware elevated-surface tokens where needed for a coherent dark shell.
- Add focused browser verification for theme switching and reload persistence.

## Out Of Scope

- Backend profile storage or API contracts for theme preferences.
- Additional theme families beyond `Light` and `Matrix Dark`.
- A system-theme or OS auto-detect mode.
- Page-by-page visual redesign outside the shared shell and shared-surface layer.

## Constraints

- Keep the existing light palette as the default runtime state.
- Theme switching must not require navigation or reload.
- Protected-route auth, workspace switcher, and current locale behavior must remain intact.

## Feature Model

Bounded context:

- Shared frontend shell presentation.

Entities:

- `AppTheme`: runtime frontend theme identity with `light` and `matrix-dark`.
- `ThemeOption`: user-visible selector option bound to an `AppTheme`.

Policies:

- Theme state is owned entirely by the frontend runtime.
- The active theme is written to `document.documentElement.dataset.theme` and persisted to local storage.
- Shared elevated surfaces must derive their color from CSS variables rather than hardcoded `bg-white` where the theme switcher is expected to reach.

## Page And Module Surface

Authenticated shell user menu:

- Route surface: any protected route using `TronShell`
- Behavior: theme changes from the `User` menu alongside the existing language selector.

App root:

- Entry point: `tron-frontend/src/app/App.tsx`
- Behavior: mounts the theme provider before routed content.

Shared UI:

- Modules: `tron-frontend/src/shared/theme/ThemeProvider.tsx`, `tron-frontend/src/shared/layout/Shell.tsx`, `tron-frontend/src/shared/ui/*`, and `tron-frontend/src/app/styles/globals.css`
- Behavior: emit and consume theme-aware CSS tokens for light and matrix-dark surfaces.

## Repository Findings

- `tron-frontend/src/shared/layout/Shell.tsx` already owns the authenticated top-right user menu and currently renders only the locale selector under the profile card.
- `tron-frontend/src/app/styles/globals.css` currently defines a single light-theme token set at `:root` with `color-scheme: light`.
- Several shared surfaces, including shell overlays and reusable cards or tables, still use hardcoded light backgrounds such as `bg-white`, which would leave broken white panels in a dark theme unless corrected.
- Protected-route auth, workspace switcher refresh, and toast infrastructure are already in place and can be reused unchanged.

## Open Decisions

- None.
