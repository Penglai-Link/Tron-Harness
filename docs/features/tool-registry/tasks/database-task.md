# Database Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: in_progress
- execution_mode: parallel

## TDD Contract

Database changes are bounded to new internal-agent policy and assignment tables.

## Task Rows

- AC-TOOL-02 | phase: tdd | owner: backend-worker | scope: migrations and repository support for workspace internal-agent policy and explicit selected-tool assignments | command_ref: CMD-TOOL-BE-TEST | status: in_progress.

## Worktree Notes

- Database changes are owned by the backend worker and integrated by the lead.

## Integration Notes

- Tool definitions stay code-backed and must not be duplicated in mutable persistence.

## Open Decisions

- None.
