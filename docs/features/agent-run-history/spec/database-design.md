# Database Design: Agent Run History

## Database Decision

- status: approved
- storage: SQLite
- reason: run history is feature-owned persistence and follows existing repository migration style.

## Existing Schema Impact

- Existing agent runtime and invocation tables do not exist.
- Existing model, variable, skill, workspace, app, user, and event-bus schemas remain unchanged.
- New history rows reference workspace, user, app, task, and model codes as strings.

## Proposed Schema

### `agent_runs`

- `id INTEGER PRIMARY KEY`
- `run_code TEXT NOT NULL UNIQUE`
- `conversation_code TEXT`
- `workspace_code TEXT NOT NULL`
- `user_code TEXT`
- `app_code TEXT`
- `task_code TEXT`
- `model_code TEXT NOT NULL`
- `status TEXT NOT NULL`
- `outcome TEXT NOT NULL`
- `step_count INTEGER NOT NULL`
- `observation_count INTEGER NOT NULL`
- `output_preview TEXT`
- `error_code TEXT`
- `error_message TEXT`
- `started_at INTEGER NOT NULL`
- `finished_at INTEGER NOT NULL`
- `created_at INTEGER NOT NULL`

## Relationships And Cardinality

- One invocation creates at most one run history row.
- One conversation can own many run rows.
- One workspace can own many run rows.

## Migration Plan

- Add one migration constant for `agent_runs`.
- Apply the migration when `SqliteAgentRunRepository` connects.
- Do not backfill old runs because no previous run history existed.

## Query And Performance Plan

- List queries filter by optional workspace and conversation code.
- List queries sort by `created_at DESC, id DESC`.
- Index workspace and conversation query paths.
- Page size is clamped by application service.

## Data Governance

- Store bounded previews only.
- Do not store raw prompt, raw completion, raw tool output, API keys, tokens, request bodies, or response bodies.
- Error messages are bounded before persistence.

## Test Data And Fixtures

- Backend API tests invoke the agent through the existing authenticated helper.
- Tests use a local model mock server and inspect persisted history through HTTP.

## Validation Commands

- `CMD-BE-TEST` validates backend behavior through `rtk make be-test`.
- `CMD-BE-CHECK` validates compilation through `rtk make be-check`.

## Open Decisions

- None.
