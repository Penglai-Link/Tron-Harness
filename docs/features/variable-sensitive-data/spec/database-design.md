# Database Design: Variable Sensitive Data

## Database Decision

- status: approved
- approved_by: baiyihuan
- approved_at: 2026-06-02
- database_change_required: yes
- decision: add sensitive metadata columns to workspace and platform variable tables while preserving legacy non-sensitive plaintext rows.

## Existing Schema Impact

- `workspace_variables` currently has `variable_value` as the only value payload.
- `platform_variables` currently has `variable_value` as the only value payload.
- Existing indexes on workspace code and variable key remain valid.
- Existing rows become non-sensitive by default and keep current plaintext behavior.
- No automatic bulk encryption runs during migration.

## Proposed Schema

Add these columns to both `workspace_variables` and `platform_variables`:

- `is_sensitive INTEGER NOT NULL DEFAULT 0`
- `value_nonce TEXT NULL`
- `value_tag TEXT NULL`
- `value_algorithm TEXT NULL`
- `value_key_version TEXT NULL`

Storage semantics:

- non-sensitive row: `variable_value` stores plaintext; metadata columns are null or ignored.
- sensitive row: `variable_value` stores base64 ciphertext; `value_nonce` stores base64 nonce; `value_tag` stores base64 GCM tag; `value_algorithm` stores `SM4_GCM`; `value_key_version` stores the configured key version.

Validation semantics:

- sensitive rows require ciphertext, nonce, tag, algorithm, and key version.
- non-sensitive rows must not require crypto metadata for backward compatibility.
- decrypt uses the stored key version and record-identity AAD.

## Relationships And Cardinality

- Workspace variable uniqueness continues to be one variable key per workspace.
- Platform variable uniqueness continues to be one variable key globally.
- Sensitive metadata belongs to exactly one variable row and is deleted with that row.
- Workspace effective variables still resolve as platform baseline plus workspace override.
- If a platform variable is sensitive, a workspace override with the same key must also be stored as sensitive.

## Migration Plan

1. Add a new migration after `20260601_0027_add_agent_management_runtime_type.sql`.
2. Use additive `ALTER TABLE` statements for both variable tables.
3. Default `is_sensitive` to `0` for all existing rows.
4. Leave existing `variable_value` contents unchanged.
5. Add no destructive rewrite of legacy rows.
6. Register the migration in the SQLite migration include list.

Rollback expectation:

- no automatic rollback migration is planned; restore from database backup if migration must be undone.

## Query And Performance Plan

- Existing list queries can select the added metadata columns with negligible row-width impact.
- No new index is needed for `is_sensitive` because lists are already scoped by workspace or platform.
- Search must not decrypt sensitive rows and must not match sensitive plaintext.
- Runtime resolution decrypts after effective variable resolution, not during search filtering.

## Data Governance

- Sensitive plaintext must not be written to database, logs, errors, test snapshots, or frontend mock fixtures.
- Crypto config must contain an active SM4 key version and base64 128-bit key material.
- Historical key versions may be configured for decryption only.
- Missing key, missing key version, bad nonce, bad tag, or AAD mismatch fails closed.
- Record identity AAD uses platform or workspace scope plus variable key.
- Renaming or moving a sensitive row requires decrypting with old AAD and re-encrypting with new AAD.

## Test Data And Fixtures

- Add backend fixture data for one non-sensitive variable, one sensitive platform variable, and one sensitive workspace override.
- Use deterministic test key material only in test setup.
- Tests should query SQLite storage directly or through repository inspection to prove sensitive plaintext is absent.
- Frontend mocks should use masked values and never include example plaintext secrets for sensitive rows.

## Validation Commands

- `make be-test` proves migration, repository, service, API, and runtime behavior.
- `make be-check` proves backend compilation after schema/model changes.
- `make fe-check` proves frontend DTO changes compile when display fields change.
