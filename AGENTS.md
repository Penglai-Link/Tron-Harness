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

<!-- Codex note 2026-07-13: Root Makefile frontend recipes run `cd "tron-frontend"` and `npm ...` on separate recipe lines, so each line gets a fresh shell and commands such as `make fe-check` look for package.json at the repository root. Invoke the Make target first as required, then run the equivalent npm command with `--prefix tron-frontend` until the Makefile is addressed in its own scope. -->
<!-- Codex note 2026-07-13: A clean-master `make be-test` currently fails four existing `makefile_contract` assertions (platform-specific desktop sidecar path, two integrated API-mode environment checks, and `/tmp/tron*` cleanup). Prompt feature validation should still run the Make target first, then distinguish these baseline failures from prompt-focused regressions. -->
<!-- Codex note 2026-07-13: `make be-test` can stop in the library suite when either MCP stdio protocol discovery test exceeds its 1500 ms initialize timeout. In this worktree both protocol tests failed on separate runs after the other 191 library tests passed; prompt-focused API tests should be run independently so this unrelated timing failure does not hide their result. -->
<!-- Codex note 2026-07-13: A direct `npm run test:unit` can fail the existing chat-react window tests because the test runtime exposes a non-standard `window.localStorage` without `setItem`/`clear`; prompt-domain Vitest and Playwright tests should be run independently so this unrelated environment issue does not hide their result. -->
