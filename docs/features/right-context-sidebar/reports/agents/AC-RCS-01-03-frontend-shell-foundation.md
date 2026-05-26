# AC-RCS-01/02/03 Frontend Shell Foundation Evidence

## Changed Paths

- `tron-frontend/src/shared/layout/Shell.tsx`
- `tron-frontend/src/store/appShellStore.ts`
- `tron-frontend/src/i18n/locales/en-US/shell.ts`
- `tron-frontend/tests/right-context-sidebar.spec.ts`
- `docs/features/right-context-sidebar/reports/agents/AC-RCS-01-03-frontend-shell-foundation.md`

## RED Evidence

- Temporary base worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/red-evidence/frontend-shell-foundation-base`
- Base source: detached worktree from `feature/clouds3n/right-context-sidebar/base` at `3bb828661d111652b54fab40e0a0fca07ebf9c06`.
- Test setup: copied only `tron-frontend/tests/right-context-sidebar.spec.ts` from this worker branch into the temporary base worktree.
- RED command: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
- RED result: failed as expected, `3 failed`.
  - AC-RCS-01 failed because `getByTestId('tron-right-sidebar')` was not found on `/shared-shell-preview`; the base shell does not expose the shell-owned desktop right-sidebar structure or collapse/reopen controls.
  - AC-RCS-02 failed because `getByTestId('tron-mobile-right-sidebar-toggle')` was not found; the base shell does not expose a mobile right-drawer launcher.
  - AC-RCS-03 failed because `getByTestId('tron-right-sidebar').getByRole('heading', { name: 'Utility Hub' })` was not found on Chat; existing right-panel content is not rendered through the new shell-owned right-sidebar contract.
- Cleanup: removed the temporary base worktree with `rtk git worktree remove --force /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/red-evidence/frontend-shell-foundation-base`.

## GREEN Evidence

### AC-RCS-01

- Desktop shell right-panel route (`/shared-shell-preview`) renders the existing `Resource Pattern` right content in a 360px sidebar.
- Collapse removes the right sidebar content from the layout, restores main content width, shows only a fixed right-edge reopen control, and persists `rightSidebarCollapsed: true` under `tron-app-shell`.
- Reopen restores the sidebar content.

### AC-RCS-02

- Mobile shell route with right content exposes a right-drawer control.
- The drawer renders as a fixed overlay with backdrop and closes by drawer close button, backdrop click, Escape, and route change.
- Before opening, the right content is not visible below the page content.

### AC-RCS-03

- Chat keeps its shell `Utility Hub` right panel.
- Observability keeps its shell `Inspector` right panel, and closing the inspector removes stale right-sidebar controls.
- Shared Shell Preview keeps its shell `Resource Pattern` right panel.
- Dashboard and Settings show no right-sidebar controls.

## Commands

- PASS: `rtk git worktree add --detach /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/red-evidence/frontend-shell-foundation-base 3bb828661d111652b54fab40e0a0fca07ebf9c06`
  - Created a throwaway detached worktree from the base branch commit for RED evidence only.
- PASS: `rtk make fe-install`
  - Installed frontend dependencies in the temporary base worktree.
- FAIL: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - RED run in the temporary base worktree failed as expected with 3 failed tests: missing desktop shell-owned right sidebar, missing mobile drawer toggle, and missing Chat right-panel content under the new shell-owned right-sidebar locator.
- PASS: `rtk git worktree remove --force /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/red-evidence/frontend-shell-foundation-base`
  - Removed the temporary RED evidence worktree.
- FAIL: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - Initial worker-branch run could not start because this worktree was missing frontend dependencies: `@playwright/test` was unresolved.
- PASS: `rtk make fe-install`
  - Installed missing frontend dependencies in this worker worktree.
- FAIL: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - Initial run after dependency installation exposed two test assertion issues: backdrop click target was covered by the drawer center, and the settings heading is `General`.
- PASS: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - 3 tests passed.
- PASS: `rtk make fe-check`
  - TypeScript typecheck and production build passed.
  - Vite emitted an existing chunk-size warning.
- FAIL: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/right-context-sidebar --phase final`
  - Blocked by out-of-scope feature orchestration state: task rows for AC-RCS-01 through AC-RCS-05 are not final, main agent status is not final, and the validator does not count this nested `reports/agents` evidence file as a final report.

## Blockers

- Final feature-doc validation is blocked by broader feature tracking files outside this slice's write scope.
