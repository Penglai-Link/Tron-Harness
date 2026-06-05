# Review Task: Task Management

## Execution Contract

- feature_slug: task-management
- status: done
- execution_mode: hybrid

## TDD Contract

Review reuses fresh backend, frontend, and browser evidence unless stale, failed, missing, or explicitly requested again.

## Task Rows

- AC-TASK-01 | phase: review | owner: lead | scope: options and rule ownership evidence | command_ref: none | status: done.
- AC-TASK-02 | phase: review | owner: lead | scope: workspace-scoped list and kanban evidence | command_ref: none | status: done.
- AC-TASK-03 | phase: review | owner: lead | scope: create default-status evidence | command_ref: none | status: done.
- AC-TASK-04 | phase: review | owner: lead | scope: parent-child and child-role evidence | command_ref: none | status: done.
- AC-TASK-05 | phase: review | owner: lead | scope: role-incompatible status/type rejection evidence | command_ref: none | status: done.
- AC-TASK-06 | phase: review | owner: lead | scope: archive/restore fallback evidence | command_ref: none | status: done.
- AC-TASK-07 | phase: review | owner: lead | scope: browser evidence | command_ref: none | status: done.
- AC-TASK-08 | phase: review | owner: lead | scope: legacy reset and seed-removal evidence | command_ref: none | status: done.
- AC-TASK-09 | phase: review | owner: lead | scope: drag/drop browser evidence, worker merge, and regression checks | command_ref: none | status: done.

## Worktree Notes

Review is lead-owned on `feature/xinlangtao/task-card-drag-status/base` in `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/task-card-drag-status/base`.

## Integration Notes

Final review must check that old enum values no longer appear in API mode, backend reset is conditional instead of unconditional, frontend mock/demo parity stays intact, and kanban drag/drop updates use POST without breaking card click navigation.

## Open Decisions

- None.
