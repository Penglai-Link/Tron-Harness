# API Contract: EliteForge Memory Dreaming Script Foundation

## API Overview

- status: not_applicable
- decision: no backend HTTP, frontend API, generated client, or external service contract changes

The feature exposes a local command-line script, not a Tron backend API. Command behavior is specified in the feature spec and acceptance rows.

## Endpoints

No endpoints are added, changed, or removed.

## Auth And Permissions

No auth or permission model changes.

The script reads local Codex session files only when explicitly invoked. Formal runtime adoption and backend authorization are deferred to a later feature.

## Error Contract

No HTTP error contract changes.

The CLI should return typed command errors for unsupported provider, invalid resolved artifact root, malformed session input, invalid task transition, invalid temp output, and blocked finalization.

## Frontend Backend Boundary

No frontend/backend boundary changes.

Frontend UI, backend job orchestration, backend Skill Registry loading, and Memory MCP integration are out of scope for this feature.

## Database And Transaction Boundary

No database transaction boundary changes.

The script writes JSON and Markdown artifacts under `.intentforge-dreaming/` or an override artifact root.

## Mock Or Adapter Expectations

Deterministic tests use fixture Codex JSONL files and temporary artifact roots. No backend mock server, frontend mock adapter, or generated client is required.

## Open Decisions

- None.
