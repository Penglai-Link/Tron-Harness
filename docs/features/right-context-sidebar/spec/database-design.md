# Database Design: Right Context Sidebar

## Database Decision

- status: approved
- decision: no database impact

## Existing Schema Impact

No existing backend table, migration, seed fixture, SQLite schema, or API persistence model changes.

Right sidebar collapsed state is browser-local shell UI state, matching the existing left sidebar persistence model in the frontend store.

## Proposed Schema

No schema is proposed.

## Relationships And Cardinality

Not applicable.

## Migration Plan

No database migration is required.

## Query And Performance Plan

No backend queries are added.

Frontend performance considerations:

- Migrated inspectors should not trigger additional API calls solely because placement moved from page grid to shell sidebar.
- Sidebar content should remain memoized or state-scoped where current pages already use local state.
- Collapsed desktop state should avoid rendering full sidebar content when it has left the layout.

## Data Governance

No new persisted business data, audit data, or sensitive data path is introduced.

## Test Data And Fixtures

Existing frontend mock data and Playwright route fixtures are sufficient.

## Validation Commands

- `CMD-FE-CHECK`
- `CMD-FE-RIGHT-SIDEBAR-BROWSER`

## Open Decisions

- None.
