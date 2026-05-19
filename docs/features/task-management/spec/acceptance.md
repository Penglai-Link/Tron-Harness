# Acceptance: Task Management

## Acceptance Index

- AC-TASK-01 [must] [api] Backend exposes the final `Feature | Bug` task types, the superpower status enum, and role-based `statusRules` plus `defaultStatuses` from `/api/v1/tasks/options`; proof: `rtk make be-test`.
- AC-TASK-02 [must] [business] List and kanban remain workspace-scoped, searchable, filterable, and hide archived tasks by default while projecting the new status universe; proof: `rtk make be-test`.
- AC-TASK-03 [must] [business] Create validates active workspace and effective assignees, generates `taskCode`, and assigns backend-owned default status `Open` based on the inferred task role; proof: `rtk make be-test`.
- AC-TASK-04 [must] [business] Detail and subtask flows preserve same-workspace parent rules, exclude archived subtasks from parent detail/statistics, and keep feature-child role inference tied to parent presence; proof: `rtk make be-test`.
- AC-TASK-05 [must] [business] Status updates and task-type edits reject role-incompatible states with `TASK_STATUS_ROLE_INVALID`, while valid updates keep progress and archived-mutation rules intact; proof: `rtk make be-test`.
- AC-TASK-06 [must] [business] Archive stores previous status, blocks parent archive while active subtasks exist, and restore returns to the stored active status or fallback `Open`; proof: `rtk make be-test`.
- AC-TASK-07 [must] [interaction] Frontend create, detail, and subtask flows use role-aware task types and statuses without surfacing old enum values; proof: Playwright task workflow.
- AC-TASK-08 [must] [business] Legacy backend task schema/data is reset once and backend task seeds are removed while frontend mock/demo parity remains available; proof: `rtk make be-test`.

## Open Decisions

- None.
