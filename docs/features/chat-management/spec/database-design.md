# Database Design: Chat Management

## Database Decision

- status: approved
- impact: create two durable chat tables and reuse existing `agent_runs`
- approved_by: implementation request
- approved_at: 2026-05-27

## Existing Schema Impact

- Existing `agent_runs` remains the execution-history table.
- New chat tables use `session_code` as the public conversation key.
- `chat_sessions` now stores optional `agent_code` to persist session-level invocation routing.
- Existing migrations are append-only; no data backfill is required for sample chat pages.

## Proposed Schema

### chat_sessions

- change_type: create
- fields: `id`, `session_code`, `workspace_code`, `user_code`, `app_code`, `task_code`, `agent_code`, `title`, `status`, `last_run_code`, `last_run_status`, `created_at`, `updated_at`
- constraints: primary key on `id`, unique `session_code`, required `workspace_code`, required `title`
- indexes: `(workspace_code, updated_at DESC, id DESC)`, `(task_code, updated_at DESC, id DESC)`
- notes: `task_code` nullable sessions are grouped as `Unassigned` in API response; nullable `agent_code` defaults to `main-agent` behavior at runtime.

### chat_messages

- change_type: create
- fields: `id`, `message_code`, `session_code`, `run_code`, `role`, `channel`, `actor`, `content`, `created_at`
- constraints: primary key on `id`, unique `message_code`, required `session_code`, required `role`, required `content`
- indexes: `(session_code, created_at ASC, id ASC)`, `(run_code, created_at ASC)`
- notes: content is intentionally persisted as durable chat history.

## Relationships And Cardinality

- One chat session has many chat messages.
- One chat session has many agent runs through `agent_runs.conversation_code=session_code`.
- One agent run may have zero or more persisted chat messages attached by `run_code`.
- No hard foreign key to `agent_runs` is required because messages can be written before a run exists.

## Migration Plan

Add `20260527_0018_create_chat_management.sql` and include it in SQLite migration constants. Add `20260530_0023_add_chat_session_agent_code.sql` for existing environments. Repository startup executes create/alter migrations with column guard checks.

## Query And Performance Plan

- Session sidebar reads sessions filtered by workspace and sorted by `updated_at DESC`.
- Message replay reads by `session_code` sorted ascending.
- Chat send performs short inserts and updates; the agent call remains outside a long database transaction.

## Data Governance

- Routes are protected by existing auth middleware.
- Workspace code is stored on sessions for tenancy filtering.
- Messages may contain user prompts and assistant outputs, so raw message reads are available only through protected chat APIs.
- API error responses must not include model API keys or raw request bodies.

## Test Data And Fixtures

- Backend tests create sessions through protected chat APIs.
- Agent send tests use the existing local mock model server pattern.
- No global seed data is required.

## Validation Commands

- `rtk make be-test`: verify migration, persistence, API routes, and orchestration.
