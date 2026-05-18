# Feature Evolution Log

## Summary

- feature_slug: workspace-management
- workflow: feature evolution
- status: completed

## Change History

- 2026-05-16: created initial AC-only workspace for `/admin/workspace` and locked the confirmed model: single workspace tree, separate direct-user and role-based grants, effective-user union view.
- 2026-05-16: completed backend workspace hierarchy and grant APIs, SQLite migrations, frontend `/admin/workspace` route and page, Playwright smoke coverage, and canonical evidence ledgers.
- 2026-05-16: redesigned the right-side workspace inspector into a single `Workspace Studio` with segmented `Overview`, `Access`, and `Create` modes to reduce information density, then refreshed smoke coverage, docs, and live browser screenshots.
- 2026-05-16: removed the top statistics cards, stripped repeated workspace-studio helper copy and summary tags, dropped the `Updated` overview stat, and stacked direct-user plus role access into full-width rows.
- 2026-05-16: moved `Move parent` below `Name`, removed the extra access title and helper descriptions, and standardized the access-card actions to right-aligned `Add` and `Save` controls.
- 2026-05-18: changed workspace creation so `workspaceCode` is server-generated as a unique random `WS-` code, removed the frontend code input, and refreshed backend plus browser evidence.
- 2026-05-18: simplified workspace-management visible button labels to verb-only actions while preserving the surrounding page context and browser coverage.
- 2026-05-18: added a clear x control to workspace search so clearing a filter triggers a fresh empty active-tree request.
- 2026-05-18: added `/api/v1/admin/workspaces/options` so task workspace dropdowns can consume lineage-labeled selector records without fetching the full tree.
- 2026-05-18: added `/api/v1/admin/workspaces/switcher` and switched the top-left shell workspace switcher to grouped workspace-management data in API mode.

## Downstream Artifact Migration

- `/admin/workspace` moved from a reserved navigation target into a real routed page and now depends on workspace API bindings, models, mock data, and Playwright proof.
- Backend admin routing now includes workspace hierarchy, access-grant, effective-user, statistics, and archive or restore endpoints.
- SQLite persistence now depends on workspace hierarchy and access-grant migrations plus repository wiring.
- Workspace create payloads now carry `name`, optional `description`, and optional `parentWorkspaceCode`; downstream callers must read `workspaceCode` from the create response.
- Workspace page button copy now carries only the action verb; downstream browser selectors scope repeated verbs to the relevant section context.
- Workspace tree search now exposes an explicit clear control; downstream browser selectors assert the empty active-tree request after clearing.
- Task workspace selectors now depend on `/api/v1/admin/workspaces/options` and no longer flatten the full workspace tree in API mode.
- The shell workspace switcher now depends on `/api/v1/admin/workspaces/switcher` in API mode and falls back to shared mock data only when the projection is unavailable.

## Residual Risks

- No validated product defects remain in the current worktree.
- Operational note: `rtk git rev-list --left-right --count master...HEAD` returned `0 0`, so the finished feature still exists only as local worktree changes until the user chooses to commit or merge it.
