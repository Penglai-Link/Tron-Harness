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
If you encounter anything counterintuitive, surprising or confusing in this project, please append it to this file as a comment.

<!-- Codex note 2026-06-29: The desktop right sidebar can keep rightPanel content mounted while collapsed through the resizable panel. Browser tests that assert chat right-panel text should first open it via the topbar toggle or `tron:open-right-sidebar`, then scope assertions to `[data-testid="tron-right-sidebar"]` instead of the whole `body`. -->
<!-- Codex note 2026-07-08: During a dev-api restart shutdown, backend logs showed `short-term memory sqlite goal conflict: UNIQUE constraint failed: agent_session_goals.session_code`; after shutdown, ports 8080/5173 released normally. -->
