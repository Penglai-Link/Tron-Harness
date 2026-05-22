# Database Design: App Management

## Database Decision

- status: approved
- impact: yes
- approved_by: clouds3n
- approved_at: 2026-05-19

SQLite remains the backing store for this slice. App management adds durable registry storage and reuses the existing task-app link table instead of redesigning task ownership persistence.

## Existing Schema Impact

Before this slice, `tron-backend` had no app registry table, app repository, or app lifecycle migration. Task persistence already stored task-to-app business links in `task_app_links`, but those links were not validated against a real app registry.

Impacted existing paths:

- `task_app_links` remains in place and keeps the task-to-app association shape.
- Task write validation in `TaskService` now depends on `AppService::assignable_app_codes`.
- Browser and task fixtures now need seeded active and archived app data instead of free-text app codes only.

## Proposed Schema

### Table: apps

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| id | INTEGER | yes | primary key | Internal row identifier. |
| app_code | TEXT | yes | unique | Stable external app identifier. |
| workspace_code | TEXT | yes | indexed | Workspace tenancy boundary from shell selection. |
| app_name | TEXT | yes | unique per normalized `workspace_code + app_name` | Operator-facing app name. |
| description | TEXT | no | none | Optional app description. |
| runtime_role | TEXT | yes | check in `frontend`, `backend` | App runtime role for this version. |
| repository_url | TEXT | no | none | Optional repository metadata. |
| default_branch | TEXT | no | none | Optional default branch metadata. |
| status | TEXT | yes | check in `active`, `archived` | App lifecycle status. |
| created_at | INTEGER | yes | unix epoch milliseconds | Creation timestamp. |
| updated_at | INTEGER | yes | unix epoch milliseconds | Last update timestamp. |
| archived_at | INTEGER | no | required when archived | Archive timestamp. |

Indexes and constraints:

- `uk_apps_app_code` unique on `app_code`.
- `uk_apps_workspace_name` unique on normalized `workspace_code`, normalized `app_name`.
- `idx_apps_workspace_status_updated_at` on `workspace_code`, `status`, `updated_at`.
- `idx_apps_workspace_role` on `workspace_code`, `runtime_role`.

### Table: app_tags

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| id | INTEGER | yes | primary key | Internal row identifier. |
| app_code | TEXT | yes | references `apps.app_code` | Business-key link to owning app. |
| tag_name | TEXT | yes | unique per `app_code + tag_name` | Lightweight operator tag. |
| linked_at | INTEGER | yes | unix epoch milliseconds | Tag creation timestamp. |

Indexes and constraints:

- `uk_app_tags_app_tag` unique on `app_code`, `tag_name`.
- `idx_app_tags_app_code` on `app_code`.

### Reused Table: task_app_links

| field | change_type | notes |
| --- | --- | --- |
| existing schema | none | Table shape remains unchanged. |
| write validation | logic update | Task writes must now reference active same-workspace app codes before persistence. |

## Relationships And Cardinality

- One workspace owns many apps through `workspace_code`.
- One app owns many tags through `app_code`.
- One task may link to many apps through existing `task_app_links`.
- One app may be linked by many tasks through existing `task_app_links`.

Domain invariants:

- `app_name` is unique only within one workspace.
- `runtime_role` is limited to `Frontend | Backend`.
- `status` is limited to `Active | Archived`.
- Archived apps remain readable but are excluded from task selection options.
- Restore reactivates the same `app_code`; it does not create a replacement record.

## Migration Plan

- migration file: `tron-backend/migrations/20260519_0011_create_app_management_apps.sql`
- order:
  - create `apps`
  - create `app_tags`
  - leave `task_app_links` unchanged
  - seed active and archived app rows used by backend tests and frontend mocks
- backward compatibility:
  - existing task rows remain valid as stored data
  - invalid legacy app codes are tolerated on read but rejected when a user explicitly changes app selection
- rollback:
  - drop `app_tags`
  - drop `apps`
  - do not mutate `task_app_links`

## Query And Performance Plan

Primary query paths:

- workspace-scoped app list with search, runtime-role filter, status filter, and pagination
- active app option list for task forms
- app detail by `app_code`
- duplicate-name detection within one workspace

Performance notes:

- list queries are bounded by workspace tenancy first
- default list excludes archived rows, reducing normal-page scan size
- option queries return active rows only and avoid loading archived records
- normalized workspace/name uniqueness prevents duplicate create/update scans in application code

## Data Governance

- Workspace tenancy is enforced by `workspace_code`.
- No hard delete is added in this version; archived rows remain auditable.
- Repository metadata is optional operator metadata, not a source of truth for filesystem paths or deployment state.
- This slice adds no new permission system; it stays inside the existing admin shell/API boundary.

## Test Data And Fixtures

Seeded registry fixtures used by API and browser verification:

- `APP-TRON-CONSOLE` active frontend app in `TRON_CORE`
- `APP-TRON-API` active backend app in `TRON_CORE`
- `APP-COMPLIANCE-API` active backend app in `COMPLIANCE`
- `APP-TRON-LEGACY` archived frontend app in `TRON_CORE`

Task fixture coverage includes:

- valid same-workspace app selection
- foreign-workspace app rejection
- archived app rejection
- legacy non-registry app code visibility on task detail

## Validation Commands

- `make be-test`: verify app persistence, lifecycle, options, and task app validation; expect: backend integration tests pass.
- `python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/app-management --phase final`: verify AC workspace documentation; expect: final validator passes.
