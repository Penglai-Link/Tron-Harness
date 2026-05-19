# Test Report: Shell Theme Switcher

## Summary

- The shell-theme-switcher proof plan is complete.
- Frontend dependency install, typecheck, build, and the dedicated Playwright theme test all passed on 2026-05-18.
- Playwright verified both immediate theme switching and reload persistence against a mounted authenticated shell with focused auth and workspace-switcher mocks.

## Evidence Rows

- AC-THEME-01 | red: there was no theme selector in the user menu and no runtime state that could change the mounted shell palette. | green: `rtk make fe-check` passed and Playwright proved the selector opens in the authenticated user menu, starts at `Light`, switches to `Matrix Dark`, and can return to `Light` without route navigation. | command_ref: CMD-FE-THEME-BROWSER | browser: clicks=sign in, open user menu, choose `Matrix Dark`, choose `Light`; inputs=theme select values `matrix-dark` and `light`; network=auth bootstrap plus workspace-switcher requests remain stable while theme changes send no backend preference write; feedback=body and header colors change immediately and the selector value updates in place; artifacts=`tron-frontend/tests/shell-theme-switcher.spec.ts`, `tron-frontend/src/shared/layout/Shell.tsx`, `tron-frontend/src/shared/theme/ThemeProvider.tsx` | status: done.
- AC-THEME-02 | red: a dark theme would not survive reload and shared surfaces would still be stuck on hardcoded light fills. | green: Playwright proved reload preserves `Matrix Dark` via `tron.theme`, and live Chrome verification confirmed the same behavior on the local backend-backed shell; shared shell cards and overlays remained dark after reload. | command_ref: CMD-FE-THEME-BROWSER | browser: clicks=sign in, open user menu, choose `Matrix Dark`, reload, reopen user menu; inputs=theme select value `matrix-dark`; network=no theme-change request fired and the existing shell requests still completed after reload; feedback=the dashboard remains matrix-dark after reload and the user menu still shows `Matrix Dark`; artifacts=`tron-frontend/tests/shell-theme-switcher.spec.ts`, `tron-frontend/src/app/styles/globals.css`, `tron-frontend/src/shared/ui/DataTable.tsx`, `tron-frontend/src/shared/ui/Toast.tsx` | status: done.

## Commands

- `rtk make fe-install` passed on 2026-05-18.
- `rtk make fe-check` passed on 2026-05-18.
- `rtk npm --prefix tron-frontend exec playwright -- test shell-theme-switcher.spec.ts --reporter=line` passed `1/1` on 2026-05-18.

## Changed Paths

- `docs/features/shell-theme-switcher/*`
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

## Blockers

- None.
