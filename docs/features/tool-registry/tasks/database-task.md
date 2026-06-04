# Database Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: not_applicable
- execution_mode: solo

## TDD Contract

Database changes are bounded to new internal-agent policy and assignment tables.

## Task Rows

- AC-TOOL-02 | phase: support | owner: lead | scope: existing internal-agent policy persistence already covered the reopened coding-domain slice, so no new database change was required | command_ref: CMD-DOCS-FINAL | status: not_applicable.

## Worktree Notes

- No database migration or repository schema change was required for this coding-domain evolution.

## Integration Notes

- Tool definitions stay code-backed and must not be duplicated in mutable persistence.

## Open Decisions

- None.
