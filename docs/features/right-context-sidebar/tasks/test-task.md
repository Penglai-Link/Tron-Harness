# Test Task: Right Context Sidebar

## Execution Contract

- feature_slug: right-context-sidebar
- status: done
- execution_mode: hybrid
- canonical_report_target: reports/test-report.md

## TDD Contract

Verification must produce concrete browser evidence for each AC. Slow browser commands are not rerun during review unless missing, stale, failed, or affected paths change.

## Task Rows

- AC-RCS-01 | phase: verify | owner: test | scope: `tron-frontend/tests/right-context-sidebar.spec.ts` desktop collapse, full-width recovery, right-edge reopen, and persistence checks | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: done.
- AC-RCS-02 | phase: verify | owner: test | scope: `tron-frontend/tests/right-context-sidebar.spec.ts` mobile drawer open and close paths | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: done.
- AC-RCS-03 | phase: verify | owner: test | scope: `tron-frontend/tests/right-context-sidebar.spec.ts`, `tron-frontend/tests/shell-theme-switcher.spec.ts`, `tron-frontend/tests/shared-ui-feedback.spec.ts` rightPanel regression and exclusion checks | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: done.
- AC-RCS-04 | phase: verify | owner: test | scope: graph and memory routes in `tron-frontend/tests/right-context-sidebar.spec.ts` | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: done.
- AC-RCS-05 | phase: verify | owner: test | scope: migrated detail and management routes in `tron-frontend/tests/right-context-sidebar.spec.ts` | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER | status: done.

## Worktree Notes

- AC-RCS-01 through AC-RCS-03 browser verification passed in the base worktree after shell foundation integration.
- AC-RCS-04 browser verification is owned by `frontend/graph-memory-migration`.
- AC-RCS-04 focused browser verification passed in the base worktree after graph and memory migration integration.
- AC-RCS-05 browser verification is owned by `frontend/management-migration`.
- AC-RCS-05 focused browser verification passed in the base worktree after management migration integration.

## Integration Notes

- Test fixtures should reuse existing mock routing style from current frontend smoke specs.
- Browser evidence must include visible right sidebar or drawer state, collapse/reopen clicks, and route-specific context assertions.
- AC-RCS-05 verification waits for AC-RCS-04 integration so selectors and assertions follow the first migrated inspector pattern.

## Open Decisions

- None.
