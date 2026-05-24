# Database Design: Model Management

## Database Decision

- status: approved
- Keep the current model-management tables, but rewrite workspace row relation semantics to the latest contract only.
- Do not preserve legacy workspace platform shadow or restore rows.

## Existing Schema Impact

- The `models` table already stores platform rows and workspace rows with `source_model_code`.
- The `workspace_default_models` table already stores one default per workspace and `modelType`.
- The current storage values still reflect an older platform-shadow model and must be reset to the latest relation kinds.

## Proposed Schema

- Reuse `models` with these workspace relation kinds:
  - `platform`: platform root model
  - `workspace_local`: workspace-owned root model
  - `workspace_override`: workspace-owned definition override for one upstream slot
  - `workspace_status_patch`: workspace-owned status-only patch for one upstream slot
- Keep `source_model_code` as the targeted upstream slot key for override or status-patch rows.
- Keep `allow_inheritance` only for `workspace_local` rows.
- Keep workspace and platform default tables, but store only visible slot `modelCode` values.

## Relationships And Cardinality

- One platform root model may be inherited by many workspaces.
- One workspace-local root model may be inherited by many descendant workspaces when `allow_inheritance = true`.
- One workspace may own at most one effective override-or-status row per upstream slot.
- One workspace default per `modelType` points at one visible slot key, regardless of whether the slot currently resolves to inherited or override state.

## Migration Plan

- Run an idempotent cleanup step on repository startup that deletes legacy workspace rows using old platform-shadow semantics.
- Remove invalid workspace defaults that reference rows deleted by the cleanup step.
- Do not attempt to translate old restore/remove compatibility data into the new model.

## Query And Performance Plan

- Effective catalog resolution scans platform roots, lineage-local roots, and lineage override or status rows for the active workspace path.
- `source_model_code` remains indexed for slot-targeted override and status lookups.
- Default resolution uses the already materialized effective catalog plus lineage-ordered workspace default assignments.

## Data Governance

- Platform API keys remain masked at the view layer.
- Workspace override and status rows are internal implementation detail records; the UI exposes only the effective row and latest semantic source state.
- Latest-only cleanup may delete obsolete workspace shadow data; that is intentional.

## Test Data And Fixtures

- Tests should create fresh platform roots, workspace-local roots, and ancestor overrides rather than relying on legacy shadow fixtures.
- Focused tests should prove both status-only patches and definition overrides on the same visible slot.

## Validation Commands

- `CMD-BE-CHECK`
- `CMD-MODEL-BE-TEST`
