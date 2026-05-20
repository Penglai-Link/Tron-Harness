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
