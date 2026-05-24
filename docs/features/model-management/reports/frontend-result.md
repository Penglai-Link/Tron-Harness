# Frontend Report: model-management

## Summary

Frontend model management is complete. `/models` now renders the same lineage semantics as the backend, with `Source = Local / Inherited / Override`, a dedicated `Inherited From` column, inherited-row edit and toggle flows, and override revert behavior. The workspace detail page can edit inherited rows into overrides, and the frontend mock layer plus Playwright smoke now follow the same latest-only contract.

## Evidence Rows

- AC-MODEL-01 | red: the workspace list still mapped platform-derived rows directly to `Override`, and `Inherited From` could not distinguish nearest-upstream workspace lineage from platform origin. | green: `/models` now renders `Local`, `Inherited`, and `Override` with a separate `Inherited From` column, and browser smoke proves parent-workspace inheritance shows the correct upstream workspace label; `rtk npm --prefix tron-frontend exec playwright -- test model-management-smoke.spec.ts --reporter=line` passed `6 passed`. | command_ref: CMD-MODEL-BROWSER | browser: clicks=open `/models` in `TRON_CORE`; inputs=seed one inheritable parent workspace row in smoke state; network=`GET /api/v1/models/workspaces/TRON_CORE` returned local and inherited rows with the new source kinds; feedback=the page visibly shows `Parent Shared Writer` as `Inherited` from `Platform` and `Core Fast` as `Local`; artifacts=`tron-frontend/src/domain/model/pages/ModelRegistryPage.tsx`, `tron-frontend/tests/model-management-smoke.spec.ts` | status: done.
- AC-MODEL-02 | red: inherited rows still exposed delete/remove behavior, disabling them re-labeled the row as `Override`, or detail-page verify still refused dirty drafts. | green: inherited rows no longer expose delete or revert actions, inherited disable keeps the row visible as `Inherited`, editing an inherited platform row creates a true `Override` that can be reverted, and both workspace plus platform detail pages can verify dirty drafts before save while keeping the saved record unchanged; `rtk npm --prefix tron-frontend exec playwright -- test model-management-smoke.spec.ts --reporter=line` passed `6 passed`. | command_ref: CMD-MODEL-BROWSER | browser: clicks=open `/models`, open row menu for `GPT-4o Main`, disable the row, open the row detail, edit and save, click `Revert`, reopen detail edit mode, click `Verify` before save, then cancel; inputs=change `Display Name` to `GPT-4o Workspace`, `GPT-4o Draft Verify`, and `GPT-4o Platform Draft Verify`; network=disable, patch, delete, and draft verify requests follow the latest-only semantics; feedback=the row remains `Inherited` after disable, becomes `Override` after save, returns to `Inherited` after revert, and unsaved detail-page verify updates the verify card without persisting the draft; artifacts=`tron-frontend/src/domain/model/pages/ModelRegistryPage.tsx`, `tron-frontend/src/domain/model/pages/WorkspaceModelDetailsPage.tsx`, `tron-frontend/src/domain/model/pages/PlatformModelDetailsPage.tsx`, `tron-frontend/tests/model-management-smoke.spec.ts` | status: done.
- AC-MODEL-03 | red: workspace defaults still behaved like row-instance defaults, so inherited -> override -> revert on the same slot could silently drop the chosen workspace default. | green: browser smoke sets a workspace default on an inherited row, edits that slot into an override, and confirms the `Default` badge survives both the override and the revert because the default follows the slot code; `rtk npm --prefix tron-frontend exec playwright -- test model-management-smoke.spec.ts --reporter=line` passed `6 passed`. | command_ref: CMD-MODEL-BROWSER | browser: clicks=open `/models`, set default on the shared inherited slot, edit the row in detail, save, then click `Revert`; inputs=change `Display Name` to a workspace-specific label; network=set-default, patch, and delete requests all target the same visible slot `modelCode`; feedback=`Default` remains visible on the slot before override, during override, and after revert; artifacts=`tron-frontend/src/domain/model/pages/ModelRegistryPage.tsx`, `tron-frontend/src/domain/model/pages/WorkspaceModelDetailsPage.tsx`, `tron-frontend/tests/model-management-smoke.spec.ts` | status: done.
- AC-MODEL-04 | red: frontend types, mocks, and smoke still encoded `WORKSPACE_LOCAL` or `WORKSPACE_INHERITED` and kept old restore/remove-shadow assumptions alive. | green: frontend types now use `PLATFORM | LOCAL | INHERITED | OVERRIDE`, the mock state was rewritten around roots, overrides, and status patches, and typecheck plus smoke are green; `rtk npm --prefix tron-frontend run typecheck` and `rtk npm --prefix tron-frontend exec playwright -- test model-management-smoke.spec.ts --reporter=line` passed. | command_ref: CMD-FE-CHECK | browser: clicks=smoke exercises `/models` and `/settings/models`; inputs=workspace disable, edit, revert, and default-selection flows; network=mocked API responses expose only the new source kinds and actions; feedback=the active UI no longer depends on old restore/shadow semantics; artifacts=`tron-frontend/src/domain/model/model/ModelRegistryModel.ts`, `tron-frontend/src/domain/model/mock/data.ts`, `tron-frontend/tests/model-management-smoke.spec.ts` | status: done.

## Changed Paths

- `tron-frontend/src/domain/model/model/ModelRegistryModel.ts`
- `tron-frontend/src/domain/model/constants/modelPageOptions.ts`
- `tron-frontend/src/domain/model/pages/ModelRegistryPage.tsx`
- `tron-frontend/src/domain/model/pages/WorkspaceModelDetailsPage.tsx`
- `tron-frontend/src/domain/model/pages/workspaceModelPageHelpers.ts`
- `tron-frontend/src/domain/model/mock/data.ts`
- `tron-frontend/tests/model-management-smoke.spec.ts`

## Commands

- `rtk make fe-check` passed on 2026-05-23.
- `rtk npm --prefix tron-frontend exec playwright -- test model-management-smoke.spec.ts --reporter=line` passed with `6 passed` on 2026-05-24.

## Blockers

- None.
