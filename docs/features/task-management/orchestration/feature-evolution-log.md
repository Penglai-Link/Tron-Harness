# Feature Evolution Log: Task Management

## Summary

- feature_slug: task-management
- evolution_status: done

## Change History

- Evolution 2026-05-19: the feature moved from the original generic task lifecycle to a superpower-aligned role-aware status model. The affected gates are feature model, acceptance, API contract, database design, backend persistence, frontend task flows, smoke coverage, and evidence ledgers.
- The change intentionally narrows task types from `Feature | BugFix | Refactor` to `Feature | Bug` and replaces the old global status list with role-specific status sets inferred from `taskType` and `parentTaskCode`.
- Because the project is still in development stage, historical backend task rows do not need compatibility migration. The evolution adopts a one-time legacy-schema rebuild path and removes backend task seed data.
- Evolution 2026-06-05: the feature adds task-card drag/drop status updates on the kanban board. The affected gates are page/module surface, API client method selection, interaction acceptance, browser proof rows, frontend task execution, worktree orchestration, and evidence ledgers.
- The change uses the existing backend status route `POST /api/v1/tasks/{taskCode}/status`; it does not add backend endpoints or alter backend status/progress rules.

## Downstream Artifact Migration

- `spec/feature-spec.md`, `acceptance.md`, `api-contract.md`, `test-plan.md`, and `database-design.md` must be rewritten to the new role-aware model.
- `tasks/*.md` and `orchestration/*.md` shifted from the original import/export-focused v1 closure to the current superpower status evolution and development reset slice.
- Backend domain, application, SQLite schema/bootstrap logic, frontend task model/pages, mock data, and Playwright smoke coverage were updated to the final contract.
- `spec/feature-spec.md`, `acceptance.md`, `api-contract.md`, `test-plan.md`, `tasks/frontend-task.md`, `tasks/test-task.md`, `tasks/review-task.md`, `orchestration/worktree-plan.md`, and evidence reports are reopened for AC-TASK-09.

## Residual Risks

- The task UI still does not implement strict adjacent transition graphs, execution engine wiring, or superpower orchestration evidence inside the product surface.
- Native HTML5 drag/drop is desktop-browser oriented; touch-first drag support remains out of scope for this evolution.
- Because migration tracking is not built into the repository bootstrap, the legacy reset must remain tightly scoped to legacy-schema detection.
