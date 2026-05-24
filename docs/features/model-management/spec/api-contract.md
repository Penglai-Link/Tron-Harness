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

Active workspace model endpoints:

- `GET /api/v1/models/workspaces/{workspaceCode}`: list the effective workspace catalog.
- `POST /api/v1/models/workspaces/{workspaceCode}`: create a workspace-local root model.
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
- `set-default` still requires the visible target row to be enabled.

## Frontend Backend Boundary

- The workspace UI must render `Source` from the current workspace perspective only: `Local`, `Inherited`, or `Override`.
- The workspace UI must render `Inherited From` separately from `Source`.
- Workspace model records must expose `sourceWorkspacePath: [{ workspaceCode, name }]` for rows whose upstream effective source is a workspace; the array is ordered root-to-source, platform-sourced rows use an empty array, and current-workspace `Local` rows do not need a path.
- Platform-sourced inherited rows keep `sourceWorkspacePath = []` and render `Inherited From = Platform`.
- A status-only local toggle on an inherited slot must not cause the API to relabel the row source as `Override`.
- The workspace UI must not offer inherited-row delete or remove behavior.
- `modelCode` in workspace responses identifies the effective visible slot, not an internal override row id.

## Database And Transaction Boundary

- The latest model may reuse the existing `models`, `workspace_default_models`, and `platform_default_models` tables.
- Workspace defaults store the visible slot `modelCode`.
- Workspace override and status patch rows target upstream slots by `source_model_code`.
- No restore-shadow compatibility path is preserved in database state.

## Mock Or Adapter Expectations

- Frontend mock data must expose only latest source semantics and latest actions.
- Browser smoke must fail if `/models` still shows `Remove` for inherited rows or if the `Source` column flips to `Override` after a status-only disable.
