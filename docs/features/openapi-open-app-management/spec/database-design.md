# Database Design: OpenAPI Open-App Management

## Database Decision

- status: not_applicable
- mandatory_database_change: no
- rationale: the HMAC foundation already introduced the required schema, and this feature only extends repository queries and transactions on those existing tables.

## Existing Schema Impact

- `platform_open_apps` already stores open-app metadata and active or disabled status.
- `platform_open_app_credentials` already stores the single active secret plus historical revoked rows.
- `platform_open_app_user_grants` and `platform_open_app_capability_grants` already model the explicit grant sets required by this feature.
- No existing table currently exposes summary-count read models or transactional replacement helpers for the management flows.

## Proposed Schema

- No new table or column is proposed in this feature.
- Existing repository queries will surface summary counts, current active credential fingerprint, and full explicit grant sets from the existing schema.
- Existing unique constraints remain the source of truth for duplicate prevention during grant replacement.

## Relationships And Cardinality

- One `OpenApp` has many explicit user grants and many capability-operation grants.
- One `OpenApp` has many historical credentials but at most one active credential.
- One open-app detail read joins one app row, zero or one active credential row, zero or more user grants, and zero or more capability grants.
- One replacement transaction mutates only one grant dimension for one open-app at a time.

## Migration Plan

- No new migration file is required.
- Repository constructors continue to apply the existing HMAC foundation migration.
- Tests must prove that the management feature works correctly against the existing schema without extra DDL.

## Query And Performance Plan

- List summaries should query by `platform_open_apps` with aggregate counts from the grant tables and the active credential fingerprint from `platform_open_app_credentials`.
- Detail reads should stay bounded to one open-app plus indexed grant lookups.
- User-grant replacement deletes current rows for one `open_app_code` and inserts the next set in one transaction.
- Capability-grant replacement deletes current rows for one `open_app_code` and inserts the next set in one transaction.

## Data Governance

- Cleartext `appKey` remains write-only to create or rotate responses and is never materialized by list or detail queries.
- Stored secret ciphertext stays unchanged in the foundational credential table.
- Disabled status changes must not delete grants or historical credentials.

## Test Data And Fixtures

- Backend tests should seed at least one open-app with one active credential, multiple explicit user grants, and multiple capability-operation grants.
- Replacement tests should cover empty replacement, additive replacement, and validation failure rollback.
- No new production seed data is required.

## Validation Commands

- `CMD-BE-TEST` proves repository evolution and admin lifecycle behavior.
- `CMD-BE-CHECK` proves the backend compiles after repository and service extensions.
- `CMD-DOCS-WORKTREE` proves the workspace stays within the required feature-doc schema.
