# Test Plan: Shell Theme Switcher

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-18

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-THEME-01 | layer: browser | case: the top-right user menu exposes a Theme selector and changing it updates the mounted shell immediately | red: the user menu still exposes only locale controls or the selected value does not change `data-theme` and visible shell colors in place | green: Playwright proves the Theme selector shows `Light` first, switches to `Matrix Dark`, and updates the running shell without navigation or reload | interaction: sign in -> open user menu -> choose `Matrix Dark` -> verify dashboard colors change -> choose `Light` -> verify colors revert | command_ref: CMD-FE-THEME-BROWSER.
- AC-THEME-02 | layer: browser | case: the selected matrix-dark theme persists after reload and does not call a backend preference API | red: reload resets the shell back to light or theme change depends on a network write | green: Playwright proves reload keeps `Matrix Dark`, local storage stores `tron.theme`, and no theme API request is sent; live Chrome verification confirms the same behavior on the local backend | interaction: sign in -> open user menu -> choose `Matrix Dark` -> reload -> reopen user menu -> verify `Matrix Dark` remains selected | command_ref: CMD-FE-THEME-BROWSER.

## Command Index

- CMD-FE-INSTALL | command: rtk make fe-install | cost: medium | when: verify workspace prerequisites | rerun: only when frontend dependencies are missing in the current worktree | expect: local frontend toolchain is installed so `tsc` and Vite are available in the current worktree.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: theme provider, shell menu wiring, and shared component token changes compile and build cleanly.
- CMD-FE-THEME-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test shell-theme-switcher.spec.ts --reporter=line | cost: slow/browser | when: verify | rerun: review-if-stale | expect: the theme selector switches between `Light` and `Matrix Dark`, updates the visible shell immediately, and persists after reload.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/shell-theme-switcher --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: shell-theme-switcher feature docs validate end to end.

## Evidence Rules

- Every `AC-THEME-*` row must have final evidence in reports.
- Browser evidence must include clicks, inputs, network assertions, visible feedback, and artifact paths.
- Review may reuse fresh Playwright evidence unless relevant frontend paths change again.
