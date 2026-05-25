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
<!-- Surprising: `Button asChild` around a React Router `Link` keeps the visual button styling but the accessible role becomes `link`, not `button`; topbar controls that product and tests both treat as buttons should use real button navigation handlers instead of `asChild`. -->
<!-- Surprising: removing the deprecated workspace model restore route `/api/v1/models/workspaces/{workspaceCode}/from-platform` does not guarantee a 404, because the remaining `/{modelCode}` workspace-model path still overlaps that URL shape; a `POST` can surface as `405 Method Not Allowed`, so tests should assert the restore flow no longer succeeds instead of hard-coding only 404. -->
<!-- Surprising: row action menus that auto-close on any global scroll can disappear immediately under Playwright because `click()` first scrolls the trigger into view; if the menu should remain open after the triggering click, ignore the first brief scroll window after opening instead of treating every scroll as an intentional dismiss. -->
<!-- Surprising: `WorkspaceDetail.lineage` is ordered root -> current workspace and still includes the current workspace entry, so lineage-based model resolvers must explicitly skip `self` when traversing ancestors or they will double-apply current-workspace logic. -->
