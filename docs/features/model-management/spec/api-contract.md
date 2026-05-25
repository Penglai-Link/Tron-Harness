# API Contract: Model Management

## API Overview

- Platform settings APIs stay under `/api/v1/admin/platform/models`.
- Workspace-effective model APIs stay under `/api/v1/models/workspaces/{workspaceCode}`.
- All responses keep the uniform envelope:

```json
{
  "code": "OK",
  "message": "",
  "data": {}
}
```

## Endpoints

Platform settings model endpoints:

- `GET /api/v1/admin/platform/models`: list platform root models.
- `POST /api/v1/admin/platform/models`: create a platform root model. If the created row is enabled and the row's coarse `modelType` slot has no explicit platform default yet, the service must persist a platform default assignment to this row during the create transaction.
- `POST /api/v1/admin/platform/models/verify`: verify a create-page draft payload without persisting it.
- `GET /api/v1/admin/platform/models/{modelCode}`: read one platform root model.
- `PATCH /api/v1/admin/platform/models/{modelCode}`: update one platform root model.
- `DELETE /api/v1/admin/platform/models/{modelCode}`: delete one platform root model.
- `POST /api/v1/admin/platform/models/{modelCode}/verify`: verify the current platform model definition for the row; when the request body carries a draft payload, verify that draft without persisting it.
- `POST /api/v1/admin/platform/models/{modelCode}/duplicate`: duplicate one platform root model.
- `POST /api/v1/admin/platform/models/{modelCode}/enable`: set `enabled = true` for a platform root model.
- `POST /api/v1/admin/platform/models/{modelCode}/disable`: set `enabled = false` for a platform root model.
- `POST /api/v1/admin/platform/models/{modelCode}/set-default`: set the platform default for the row's coarse `modelType`.

Active workspace model endpoints:

- `GET /api/v1/models/workspaces/{workspaceCode}`: list the effective workspace catalog.
- `POST /api/v1/models/workspaces/{workspaceCode}`: create a workspace-local root model.
- `POST /api/v1/models/workspaces/{workspaceCode}/verify`: verify a create-page draft payload for a would-be workspace-local root model without persisting it.
- `GET /api/v1/models/workspaces/{workspaceCode}/select-options`: list visible selector options from the effective catalog.
- `GET /api/v1/models/workspaces/{workspaceCode}/{modelCode}`: read one visible effective row by slot `modelCode`.
- `PATCH /api/v1/models/workspaces/{workspaceCode}/{modelCode}`: update a workspace-local row in place, or create or update a current-workspace override for an inherited slot.
- `DELETE /api/v1/models/workspaces/{workspaceCode}/{modelCode}`: delete a current-workspace local row, or revert a current-workspace override back to inherited state.
- `POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/verify`: verify the effective visible model definition for the slot; when the request body carries a draft payload, verify that draft without persisting it.
- `POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/duplicate`: copy the effective visible row into a new current-workspace local model.
- `POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/enable`: set `enabled = true` for a local row, a current-workspace override, or a current-workspace status patch.
- `POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/disable`: set `enabled = false` for a local row, a current-workspace override, or a current-workspace status patch.
- `POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/set-default`: set the workspace default by visible slot `modelCode` for the row `modelType`.
- `POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/allow-inheritance`: change descendant inheritance only for a current-workspace local root model.

Removed latest-only exclusions:

- no workspace restore-candidate endpoint
- no workspace add-from-platform endpoint
- no workspace remove-shadow endpoint

## Auth And Permissions

- Model management remains behind the authenticated admin shell session.
- Workspace model routes require an active workspace code.

## Error Contract

- Workspace-local uniqueness conflicts keep the existing `MODEL_DISPLAY_NAME_DUPLICATE` code with a workspace-scoped message.
- `allowInheritance` remains invalid for inherited or override rows.
- `DELETE` against a pure inherited row is invalid because inherited rows are not removable; only local rows can be deleted and only current-workspace override rows can be reverted by delete.
- `set-default` still requires the target row to be enabled for both platform and workspace routes.
- Draft-verify endpoints must validate the submitted payload against the same protocol and `modelType` rules as save, but they must not allocate or persist a new row implicitly.

## Frontend Backend Boundary

- The workspace UI must render `Source` from the current workspace perspective only: `Local`, `Inherited`, or `Override`.
- The workspace UI must render `Inherited From` separately from `Source`.
- Workspace model records must expose `sourceWorkspacePath: [{ workspaceCode, name }]` for rows whose upstream effective source is a workspace; the array is ordered root-to-source, platform-sourced rows use an empty array, and current-workspace `Local` rows do not need a path.
- Platform-sourced inherited rows keep `sourceWorkspacePath = []` and render `Inherited From = Platform`.
- A status-only local toggle on an inherited slot must not cause the API to relabel the row source as `Override`.
- The workspace UI must not offer inherited-row delete or remove behavior.
- Platform and workspace create pages may call draft-verify endpoints before first save.
- Platform and workspace detail pages may send the current edit-form payload to verify without saving.
- Default slots are keyed by coarse `modelType`; UI capability labels or capability combinations must not create separate default-assignment buckets.
- `modelCode` in workspace responses identifies the effective visible slot, not an internal override row id.

## Database And Transaction Boundary

- The latest model may reuse the existing `models`, `workspace_default_models`, and `platform_default_models` tables.
- `platform_default_models` and `workspace_default_models` each store one assignment per coarse `modelType`.
- Workspace defaults store the visible slot `modelCode`.
- The first enabled platform create in an empty `modelType` slot must materialize `platform_default_models` instead of relying only on runtime fallback.
- Workspace override and status patch rows target upstream slots by `source_model_code`.
- No restore-shadow compatibility path is preserved in database state.

## Mock Or Adapter Expectations

- Frontend mock data must expose only latest source semantics and latest actions.
- Browser smoke must fail if `/models` still shows `Remove` for inherited rows or if the `Source` column flips to `Override` after a status-only disable.
