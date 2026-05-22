# Frontend Task: Auth

## Execution Contract

- feature_slug: auth
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

Only split into multiple workers when contracts are stable, write scopes are disjoint, and splitting reduces wall-clock time.

## Task Rows

- AC-AUTH-03 | phase: tdd | owner: lead | scope: tron-frontend/src/app/App.tsx; tron-frontend/src/domain/auth/*; tron-frontend/src/router/* | command_ref: CMD-FE-CHECK | status: done.
- AC-AUTH-04 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/auth/*; tron-frontend/src/i18n/locales/en-US/auth.ts | command_ref: CMD-FE-CHECK | status: done.
- AC-AUTH-08 | phase: tdd | owner: lead | scope: tron-frontend/src/app/App.tsx; tron-frontend/src/domain/auth/components/*; tron-frontend/src/domain/shared-shell/pages/SharedShellPreviewPage.tsx; tron-frontend/src/shared/layout/Shell.tsx; tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-FE-AUTH-BROWSER | status: done.
- AC-AUTH-04 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/auth/pages/LoginPage.tsx; tron-frontend/src/i18n/locales/en-US/auth.ts; tron-backend/src/config.rs; tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-FE-AUTH-BROWSER | status: done.
- AC-AUTH-08 | phase: tdd | owner: lead | scope: tron-frontend/src/shared/api/httpClient.ts; tron-frontend/src/store/authStore.ts; tron-frontend/src/domain/auth/hooks/index.ts; tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-FE-AUTH-BROWSER | status: done.

## Worktree Notes

Lead executes route-guard, auth UI, and session lifecycle work directly in `feature/clouds3n/auth/base`.

## Integration Notes

Frontend auth state must keep the access JWT in runtime memory only, attach it to protected API calls through `Authorization: Bearer`, rebuild that runtime state from the refresh-cookie contract on page load or OAuth callback, renew idle access before expiry, and recover the first expired protected request without redirecting the operator away from in-progress work.
Current evolution pass tightens the login surface to the reviewed single-card layout, aligns the live external provider with GitHub, removes the old access-cookie bootstrap path entirely, and adds proactive renewal plus stale-token retry at the shared auth transport layer.

## Open Decisions

- None.
