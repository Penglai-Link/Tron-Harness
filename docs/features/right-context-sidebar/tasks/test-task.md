# Test Task: Right Context Sidebar

## Execution Contract

- feature_slug: right-context-sidebar
- status: waiting_human_decision
- execution_mode: hybrid
- canonical_report_target: reports/test-report.md

## TDD Contract

Verification must produce concrete browser evidence for each AC. Slow browser commands are not rerun during review unless missing, stale, failed, or affected paths change.

## Task Rows

- AC-RCS-01 | phase: verify | owner: test | scope: `tron-frontend/tests/right-context-sidebar.spec.ts` desktop collapse, full-width recovery, right-edge reopen, and persistence checks | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: waiting_human_decision.
- AC-RCS-02 | phase: verify | owner: test | scope: `tron-frontend/tests/right-context-sidebar.spec.ts` mobile drawer open and close paths | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: waiting_human_decision.
- AC-RCS-03 | phase: verify | owner: test | scope: `tron-frontend/tests/right-context-sidebar.spec.ts`, `tron-frontend/tests/shell-theme-switcher.spec.ts`, `tron-frontend/tests/shared-ui-feedback.spec.ts` rightPanel regression and exclusion checks | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: waiting_human_decision.
- AC-RCS-04 | phase: verify | owner: test | scope: graph and memory routes in `tron-frontend/tests/right-context-sidebar.spec.ts` | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: waiting_human_decision.
- AC-RCS-05 | phase: verify | owner: test | scope: migrated detail and management routes in `tron-frontend/tests/right-context-sidebar.spec.ts` | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: waiting_human_decision.

## Worktree Notes

- Browser verification should run from the implementation worktree after GREEN evidence exists.

## Integration Notes

- Test fixtures should reuse existing mock routing style from current frontend smoke specs.
- Browser evidence must include visible right sidebar or drawer state, collapse/reopen clicks, and route-specific context assertions.

## Open Decisions

- Execution approval is pending.
