# Database Design: EliteForge Memory Dreaming Script Foundation

## Database Decision

- status: not_applicable
- impact: none
- approved_by: user confirmed no DB/API scope in current AC set
- approved_at: 2026-05-28

This feature creates a repo-local skill source and script artifacts only. It does not add, alter, or remove backend database schema.

## Existing Schema Impact

No existing table, migration, SQLite schema, repository, ORM entity, seed data, cache, report, or persistence model changes.

## Proposed Schema

No schema is proposed.

## Relationships And Cardinality

Not applicable.

## Migration Plan

No database migration is required.

## Query And Performance Plan

No backend query path is added.

Script performance is file-system bound:

- Codex session discovery should honor time range and max session bounds.
- Deterministic tests should use small fixture sets.
- Formal and override artifact roots should avoid copying session text into `runs/<dream-run-id>/`.

## Data Governance

No database governance change.

Runtime artifact governance is file based:

- Formal run artifacts live under `.intentforge-dreaming/`.
- Development and test artifacts live under explicit override roots.
- Runtime artifacts are not source artifacts and must not be committed.
- CLI output must not print full message bodies.

## Test Data And Fixtures

Script tests use local Codex JSONL fixtures and temporary artifact roots. They do not require database seed data.

## Validation Commands

- `CMD-SKILL-MEMORY-DREAMING-TEST`: run script tests through the root Makefile.
- `CMD-DOCS-SPEC`: validate feature spec documents.

## Open Decisions

- None.
