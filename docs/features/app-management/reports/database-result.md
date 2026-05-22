# Database Report: App Management

## Summary

Added durable app registry storage with `apps` and `app_tags` while preserving `task_app_links`. Highest-risk delta was moving task app validation from free-text acceptance to active registry-backed selection.

## Evidence Rows

- AC-APP-04 | red: app lifecycle storage did not exist before the migration | green: archived apps persist in the registry and restore without replacement records | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-APP-05 | red: app list, detail, and update storage paths were absent | green: migration and SQLite repository back the new workspace-scoped app APIs | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-APP-06 | red: task app links had no registry-backed validation source | green: existing `task_app_links` reuse plus app lookup now rejects invalid selections before persistence | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/migrations/20260519_0011_create_app_management_apps.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/app_repository.rs`

## Commands

- @ref:spec/test-plan.md#command-index: `CMD-BE-TEST` passed.

## Blockers

- None.
