# Frontend Result: Shell Theme Switcher

## Summary

- The authenticated top-right user settings menu now exposes both `Theme` and `Language`, with `Light` preserved as the default and `Matrix Dark` added as the dark built-in option.
- A new shared `ThemeProvider` owns runtime theme state, writes `data-theme` to the document root, and persists the selected value to `tron.theme`.
- Shared elevated surfaces now consume theme-aware CSS variables so the Matrix palette reaches the shell, cards, tables, tabs, toggles, and toast surfaces instead of leaving large hardcoded white panels behind.

## Evidence Rows

- AC-THEME-01 | red: the user menu only exposed locale controls and the app had a single hardcoded light token set. | green: the `Theme` selector now renders in the user menu with `Light` default and `Matrix Dark` option, and changing it updates the mounted dashboard shell immediately without route navigation; `rtk make fe-check`, Playwright, and live Chrome verification all passed. | command_ref: CMD-FE-THEME-BROWSER | browser: clicks=sign in, open user menu, choose `Matrix Dark`, reopen the menu after reload, choose `Light`; inputs=theme select values `matrix-dark` and `light`; network=initial auth refresh/login and `GET /api/v1/admin/workspaces/switcher` remained stable while theme changes sent no preference API call; feedback=the shell header, sidebar, cards, and user-menu panel switch theme in place and the selector value updates immediately; artifacts=`tron-frontend/src/shared/layout/Shell.tsx`, `tron-frontend/src/shared/theme/ThemeProvider.tsx`, `tron-frontend/src/app/App.tsx`, `tron-frontend/tests/shell-theme-switcher.spec.ts` | status: done.
- AC-THEME-02 | red: switching to a dark palette would still leave shared white panels and reload would reset the app to light. | green: `Matrix Dark` now applies dark neon-green tokens across shared elevated surfaces and persists through `tron.theme` after reload; Playwright proved local-storage persistence and live Chrome verification on `http://127.0.0.1:4173/dashboard` confirmed the same protected-route behavior. | command_ref: CMD-FE-THEME-BROWSER | browser: clicks=open user menu, select `Matrix Dark`, reload, reopen user menu; inputs=theme select value `matrix-dark`; network=no theme-change request was sent and the existing shell bootstrap requests continued to succeed after reload; feedback=after switch and reload the dashboard stays dark, the user menu keeps `Matrix Dark`, and shared cards or panels remain dark instead of reverting to white; artifacts=`tron-frontend/src/app/styles/globals.css`, `tron-frontend/src/shared/layout/SurfaceCard.tsx`, `tron-frontend/src/shared/ui/DataTable.tsx`, `tron-frontend/src/shared/ui/Forms.tsx`, `tron-frontend/src/shared/ui/Tabs.tsx`, `tron-frontend/src/shared/ui/Toast.tsx`, `tron-frontend/tests/shell-theme-switcher.spec.ts` | status: done.

## Changed Paths

- `tron-frontend/src/app/App.tsx`
- `tron-frontend/src/app/styles/globals.css`
- `tron-frontend/src/domain/auth/components/AuthTopbarActions.tsx`
- `tron-frontend/src/i18n/locales/en-US/shell.ts`
- `tron-frontend/src/shared/index.ts`
- `tron-frontend/src/shared/layout/Shell.tsx`
- `tron-frontend/src/shared/layout/SurfaceCard.tsx`
- `tron-frontend/src/shared/theme/ThemeProvider.tsx`
- `tron-frontend/src/shared/ui/DataTable.tsx`
- `tron-frontend/src/shared/ui/Forms.tsx`
- `tron-frontend/src/shared/ui/Tabs.tsx`
- `tron-frontend/src/shared/ui/Toast.tsx`
- `tron-frontend/tests/shell-theme-switcher.spec.ts`

## Commands

- `rtk make fe-install` passed on 2026-05-18.
- `rtk make fe-check` passed on 2026-05-18.
- `rtk npm --prefix tron-frontend exec playwright -- test shell-theme-switcher.spec.ts --reporter=line` passed `1/1` on 2026-05-18.
- Live Chrome check on `http://127.0.0.1:4173/dashboard` passed on 2026-05-18.

## Blockers

- None.
