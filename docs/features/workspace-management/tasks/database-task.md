# Database Task: Workspace Management

## Execution Contract

- feature_slug: workspace-management
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

## Task Rows

- AC-WS-06 | phase: tdd | owner: lead | scope: tron-backend/migrations/*.sql tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs | command_ref: CMD-BE-TEST | status: done.
- AC-WS-08 | phase: tdd | owner: lead | scope: tron-backend/migrations/*.sql tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs tron-backend/tests/workspace_management_api.rs | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Workspace lifecycle and grant tables must persist under the same SQLite database used by users and roles.

## Integration Notes

- Reuse the existing SQLite migration registry and persistence bootstrap.
- Keep workspace grant persistence compatible with upstream `users`, `roles`, and `user_role_assignments`.

## Open Decisions

- None.
