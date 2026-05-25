# Database Design: Model Management

## Database Decision

- status: approved
- Keep the current model-management tables, but rewrite workspace row relation semantics to the latest contract only.
- Do not preserve legacy workspace platform shadow or restore rows.

## Existing Schema Impact

- The `models` table already stores platform rows and workspace rows with `source_model_code`.
- The `platform_default_models` table already stores one default per coarse `modelType`.
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
- Treat `modelType` as the coarse default-slot identity; capability labels such as `Text Generation` or `Deep Thinking` remain metadata inside the same `TEXT` slot.

## Relationships And Cardinality

- One platform root model may be inherited by many workspaces.
- One platform default per coarse `modelType` points at one platform root slot key.
- One workspace-local root model may be inherited by many descendant workspaces when `allow_inheritance = true`.
- One workspace may own at most one effective override-or-status row per upstream slot.
- One workspace default per `modelType` points at one visible slot key, regardless of whether the slot currently resolves to inherited or override state.

## Migration Plan

- Run an idempotent cleanup step on repository startup that deletes legacy workspace rows using old platform-shadow semantics.
- Remove invalid workspace defaults that reference rows deleted by the cleanup step.
- Do not attempt to translate old restore/remove compatibility data into the new model.
- Create flow must materialize `platform_default_models` when the first enabled platform root model appears in an empty coarse `modelType` slot instead of relying only on runtime fallback.

## Query And Performance Plan

- Effective catalog resolution scans platform roots, lineage-local roots, and lineage override or status rows for the active workspace path.
- `source_model_code` remains indexed for slot-targeted override and status lookups.
- Platform default resolution and workspace default resolution both key by coarse `modelType`, not by capability combinations.
- Default resolution uses the already materialized effective catalog plus lineage-ordered workspace default assignments.

## Data Governance

- Platform API keys remain masked at the view layer.
- Workspace override and status rows are internal implementation detail records; the UI exposes only the effective row and latest semantic source state.
- Latest-only cleanup may delete obsolete workspace shadow data; that is intentional.

## Test Data And Fixtures

- Tests should create fresh platform roots, workspace-local roots, and ancestor overrides rather than relying on legacy shadow fixtures.
- Focused tests should prove both status-only patches and definition overrides on the same visible slot.
- Focused tests should prove create-draft verify never persists a row and that the first enabled platform root in an empty `modelType` slot writes an explicit platform default assignment.

## Validation Commands

- `CMD-BE-CHECK`
- `CMD-MODEL-BE-TEST`
