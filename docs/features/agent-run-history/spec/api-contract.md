# API Contract: Agent Run History

## API Overview

Protected APIs list and inspect safe agent run history records.

## Endpoints

- `GET /api/v1/agents/runs`
  - Query: `workspaceCode?`, `conversationCode?`, `current?`, `size?`.
  - Response: paged safe run summaries.
- `GET /api/v1/agents/runs/{runCode}`
  - Response: safe run detail.

## Auth And Permissions

- Existing protected-route middleware is required.
- Returned rows are scoped by query filters in this feature.
- Deeper workspace authorization follows existing protected-route baseline.

## Error Contract

- `AGENT_RUN_NOT_FOUND`
- `AGENT_HISTORY_STORAGE_FAILED`

## Frontend Backend Boundary

- No frontend page is added.
- Existing frontend app shell must keep building and remain browser-usable.

## Database And Transaction Boundary

- Invocation writes one history row after run completion or runtime failure.
- List and detail APIs read from SQLite through the history service.
- Runtime execution does not hold database locks.

## Mock Or Adapter Expectations

- Tests use the existing authenticated API helper and local model mock.
- No frontend mock data changes are required.

## Open Decisions

- None.
