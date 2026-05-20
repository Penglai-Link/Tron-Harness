# Database Design: Automations

## Database Decision

- status: approved
- decision: store automations and execution instances in SQLite, and use `tokio-cron-scheduler` as the in-process runtime scheduler while keeping persistence authoritative in SQLite.
- rationale: the current backend already uses Axum + Tokio + SQLite, so the scheduler should stay in-process and low-ceremony while supporting async cron registration and dynamic add or remove behavior.

## Existing Schema Impact

- No existing automation tables are present in the current backend.
- Existing workspace tables remain the source of truth for workspace validity.
- Existing Snowflake-style numeric IDs and generated code conventions should be reused for automation and execution records.
- Existing task and variable repositories provide the reference pattern for migrations, timestamps, archive lifecycle fields, and workspace validation.

## Proposed Schema

- `automations`
  - `id INTEGER PRIMARY KEY`
  - `automation_code TEXT NOT NULL UNIQUE`
  - `workspace_code TEXT NOT NULL`
  - `name TEXT NOT NULL`
  - `description TEXT NULL`
  - `prompt_text TEXT NOT NULL`
  - `cron_expression TEXT NOT NULL`
  - `scheduler_expression TEXT NOT NULL`
  - `timezone TEXT NOT NULL DEFAULT 'UTC'`
  - `status TEXT NOT NULL CHECK (status IN ('Active', 'Paused', 'Archived'))`
  - `previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Active', 'Paused'))`
  - `last_execution_status TEXT NULL CHECK (last_execution_status IS NULL OR last_execution_status IN ('Pending', 'Running', 'Success', 'Failed', 'Canceled', 'Skipped'))`
  - `last_execution_code TEXT NULL`
  - `last_execution_summary TEXT NULL`
  - `last_started_at INTEGER NULL`
  - `last_finished_at INTEGER NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
  - `archived_at INTEGER NULL`
- `automation_execution_records`
  - `id INTEGER PRIMARY KEY`
  - `execution_code TEXT NOT NULL UNIQUE`
  - `automation_code TEXT NOT NULL REFERENCES automations(automation_code)`
  - `workspace_code TEXT NOT NULL`
  - `trigger_source TEXT NOT NULL CHECK (trigger_source IN ('Scheduler', 'Manual'))`
  - `status TEXT NOT NULL CHECK (status IN ('Pending', 'Running', 'Success', 'Failed', 'Canceled', 'Skipped'))`
  - `summary TEXT NULL`
  - `scheduled_at INTEGER NULL`
  - `started_at INTEGER NOT NULL`
  - `finished_at INTEGER NULL`
  - `created_at INTEGER NOT NULL`

## Relationships And Cardinality

- One workspace has many automations.
- One automation has many execution records.
- One automation keeps a denormalized rollup of the most recent execution for list and detail reads.
- Execution records keep immutable history; automation rows keep mutable operational state.

## Migration Plan

- Add a new migration file that creates `automations` and `automation_execution_records`.
- Add supporting indexes for workspace list queries, lifecycle filtering, and execution-history lookup.
- Keep task and other domain migrations untouched.
- Use the same legacy-reset style as task migrations only if a future automation legacy schema appears; no reset is needed for the initial version.

## Query And Performance Plan

- Primary list query filters by `workspace_code`, `status`, and fuzzy `search` on `automation_code`, `name`, and `description`.
- Execution-history query filters by `automation_code` and orders by newest `started_at`.
- Add indexes:
  - `idx_automations_workspace_status` on `(workspace_code, status)`
  - `idx_automations_workspace_updated` on `(workspace_code, updated_at)`
  - `idx_automation_executions_automation_started` on `(automation_code, started_at DESC)`
  - `idx_automation_executions_workspace_started` on `(workspace_code, started_at DESC)`
- Do not persist `next_run_at`; compute it from the normalized cron expression on read to avoid stale scheduler metadata writes.
- Normalize five-field cron input to a six-field scheduler expression by prefixing second `0` before scheduler registration.

## Data Governance

- `Archived` automations remain queryable only when the caller explicitly includes archived status.
- `Paused` automations are operationally inactive but still editable and restorable to `Active`.
- Execution summaries must not persist secret tokens or external credentials.
- Prompt text is stored as operator-authored content and treated as ordinary business text, not executable source code.

## Test Data And Fixtures

- Seed no persistent automation records in backend tests by default.
- Backend tests create workspace-bound automations inline and assert clean empty-state lists for unaffected workspaces.
- Frontend mock fixtures should include `Active`, `Paused`, and `Archived` examples plus multiple execution result states.

## Validation Commands

- `rtk make be-test`
- `rtk make be-check`
