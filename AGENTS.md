# AGENTS.md

## Instructions

- Always use `Makefile` first to bulid,test and cleanup
- Use a different available port if the default port is already in use, and ensure the frontend matchs the backend port

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

<!-- Review note: frontend i18n currently ships only `en-US` resources even though project docs require both `en-US` and `zh-CN` from the first production-ready implementation. -->
<!-- Investigation note: as of 2026-05-29, `feature/clouds3n/chat-agui-user-cards/base` points at the same commit as `master` while its verified implementation exists as uncommitted worktree changes; remote `feature-20260529-gwz` also points at `master` and does not follow the `feature/<developer>/...` naming rule. -->
<!-- Investigation note: as of 2026-05-29, shell/git commands may print a Homebrew vendor Ruby `cannot load such file -- ostruct` warning while still completing successfully; treat command exit status and command-specific output as authoritative. -->
<!-- Investigation note: graph and memory frontend domains already declare `/api/v1/knowledge-graph` and `/api/v1/memory/fragments` endpoint strings, but the graph storage foundation first version is explicitly backend-internal and must not add HTTP APIs. -->
