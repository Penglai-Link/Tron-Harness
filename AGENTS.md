# AGENTS.md

## Instructions

- Always use `Makefile` first.

## API

Use a uniform response body format:
```
{
  code: "response status code or error code"
  message: "error message"
  data: [] | {}
}
```

Pagable response body format:
```
{
  code: "response status code"
  message: "error message"
  data: {
    current: "current page, 1 based"
    size: "page size"
    total: "total count"
    records: [
      // data arry
    ]
  }
}
```

## Skill Constraints

- Git Spec follows `$eliteforge-git-feature-oriented-lite-spec`

## What's more
If you encounter anything counterintuitive, surprising or confusing in this project, please append it to this file as a comment.
<!-- Surprising: authenticated frontend smoke tests that mount `TronShell` must also mock `GET /api/v1/admin/workspaces/switcher`; a 401 from the shell-level workspace switcher request triggers the global unauthorized handler and silently bounces the page back to `/login`, even when the target page API itself is mocked successfully. -->
<!-- Surprising: `user_oauth_identities` already preserved GitHub provenance, but legacy single-token GitHub profiles had overloaded `last_name = GitHub`; user detail now needs a dedicated `sourceType` field and a read-time fallback cleanup so old OAuth users stop surfacing provider names as surnames. -->
<!-- Surprising: the `skills` CLI subcommands do not expose normal per-command help semantics during runtime integration; `npx skills find --help` is treated as a literal search query, and `npx skills update --help` performs an update check instead of printing help, so command wiring should rely on verified contracts rather than probing subcommand help. -->
<!-- Surprising: `AutomationDetailsPage` can observe an empty execution-history page object during create -> detail navigation while the history request settles; reading `executionPage.total` or `executionPage.records` without a safe empty-page fallback crashes the entire route before any detail fields render. -->
<!-- Surprising: automation hooks still fall back to mock data when `VITE_TRON_AUTOMATION_DATA_SOURCE=api` but the backend request fails, so API mode is not a strict no-mock guarantee unless that fallback path is removed. -->
<!-- Surprising: React StrictMode plus inline query objects in automation hooks can trigger duplicate initial `/automations` and `/executions` API requests in dev; if the request logic is mount-scoped and aborts on cleanup, the detail execution table may visibly flash during the first load even after the empty state text is removed. -->
<!-- Surprising: hard-loading built preview routes such as `/shared-shell-preview` can briefly render a nearly blank page with only the global progress bar and Main Agent FAB before the lazy route chunk resolves, so visual smoke checks should wait for shell/body text instead of trusting the first paint. -->
<!-- Surprising: admin surfaces can fire several protected requests at once against the same soon-to-expire bearer token; after one request refreshes successfully, later 401 responses may still arrive from the stale token unless the frontend retries with the newer in-memory token before attempting another refresh, otherwise auth churn can cascade into unnecessary redirects. -->
<!-- Surprising: authenticated console routes are already wrapped by `TronShell` in `src/app/App.tsx`, so page components that add their own `TronShell` create a nested shell layout instead of a standalone page frame. -->
<!-- Surprising: the legacy `/models/new` route had become a stale alias for the platform models page, so opening a workspace-scoped “Add Model” URL could silently create platform models unless the route is redirected or removed. -->
<!-- Surprising: the shared frontend layout contract is documented in `tron-frontend/docs/spec/react-coding-spec.md`, but `src/shared/layout/PageHeader.tsx` and multiple pages had drifted to support `eyebrow/status/description`; before copying an existing page pattern, verify the shared primitive against that spec file instead of assuming the implementation still matches it. -->
<!-- Surprising: `Button asChild` around a React Router `Link` keeps the visual button styling but the accessible role becomes `link`, not `button`; topbar controls that product and tests both treat as buttons should use real button navigation handlers instead of `asChild`. -->
<!-- Surprising: model verify endpoints only accept persisted `modelCode` records and never read unsaved form drafts, so any create/detail UI that offers Verify must either save first or intentionally implement create-and-verify behavior. -->
<!-- Surprising: `PlatformModelDetailsPage` reuses `fetchPlatformModels()` and selects the current row locally instead of calling a dedicated detail API, so any sensitive field exposed by the platform list response lands in page state unless the backend response and frontend model API layer both mask it before records are stored. -->
<!-- Surprising: a long-running local `tron-backend` on :8080 can keep serving an older compiled binary after request-schema changes; when the frontend starts sending new fields like `modelCapabilities`, platform model PATCH may return a deserialization 422 until the backend process is explicitly restarted from the current worktree. -->
<!-- Surprising: console-route workspace selection lives only in the in-memory Zustand shell store; using a full-page navigation like Playwright `page.goto("/models")` resets the active workspace back to the defaults, so multi-workspace UI checks must stay inside client-side navigation if they need to preserve the current workspace context. -->
<!-- Surprising: removing the deprecated workspace model restore route `/api/v1/models/workspaces/{workspaceCode}/from-platform` does not guarantee a 404, because the remaining `/{modelCode}` workspace-model path still overlaps that URL shape; a `POST` can surface as `405 Method Not Allowed`, so tests should assert the restore flow no longer succeeds instead of hard-coding only 404. -->
<!-- Surprising: removing backend model seed data does not free up the old seed `modelCode` range, because fresh empty-database model creation reuses the same deterministic snowflake sequence; cleanup migrations must fingerprint demo records by scope/name/api-key markers instead of deleting by fixed `modelCode` alone. -->
<!-- Surprising: once platform defaults and workspace overrides coexist, a workspace-level remove of an inherited platform model must only clear default assignments scoped to that workspace; deleting defaults globally by `modelCode` also silently unsets the platform default for every other workspace. -->
<!-- Surprising: model-management persistence still stamps every model row with the same placeholder `created_at`, so any “first created model wins” fallback must key off the monotonic Snowflake `id` order instead of timestamps. -->
<!-- Surprising: row action menus that auto-close on any global scroll can disappear immediately under Playwright because `click()` first scrolls the trigger into view; if the menu should remain open after the triggering click, ignore the first brief scroll window after opening instead of treating every scroll as an intentional dismiss. -->
<!-- Surprising: `WorkspaceDetail.lineage` is ordered root -> current workspace and still includes the current workspace entry, so lineage-based model resolvers must explicitly skip `self` when traversing ancestors or they will double-apply current-workspace logic. -->
<!-- Surprising: the EliteForge feature-doc validator treats the literal phrase `not runtime` in final reports as if it were the unresolved marker `Not run`, so status summaries should avoid that wording or the docs gate fails falsely. -->
