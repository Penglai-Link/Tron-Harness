# AGENTS.md

## Instructions

- Always use `Makefile` first to bulid,test and cleanup
- Use a different available port if the default port is already in use, and ensure the frontend matchs the backend port
- Always follow coding spec

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
- macOS ships GNU Make 3.81, which does not support `.ONESHELL`; keep recipes from relying on shell state across recipe lines.
- The chat management page once carried an unused `../components/terminal` import even though that module does not exist; keep package-level chat UI dependencies explicit so `make fe-check` remains runnable in a clean worktree.
- The sidebar uses the persisted navigation tree whenever the API data source is active; change the built-in SQLite menu migration as well as `shellNavigation.ts` when a default sidebar hierarchy changes.
- If you encounter anything counterintuitive, surprising or confusing in this project, please append it to this file as a comment.
- `ToggleSwitch` currently applies a hard-coded `title="12"` to every rendered label; do not treat it as user-facing copy when composing layouts around that control.
- `cargo fmt` can reformat unrelated tracked files; review and retain only feature-owned changes before committing.

<!-- Tool exposure is run-local: STICKY_EXPANDED does not persist expanded domains into a new run. EPHEMERAL_EXPANDED collapses all expanded domains after any successful non-discovery tool, without removing historical tool results. -->
<!-- The broad right-context-sidebar Playwright file contains unrelated stale page-heading assertions; use --grep AC-STM-08 for the chat STM panel until those page fixtures are updated. -->
