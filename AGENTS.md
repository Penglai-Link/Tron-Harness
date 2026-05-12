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

<!-- Surprising: Codex attached the lead agent to /Users/clouds3n/.codex/worktrees/1045/Tron, while the EliteForge Lite Spec canonical multi-agent base path for this feature would mirror feature/clouds3n/init-rust-backend/base under the worktree root. Worker worktrees should still use mirrored Lite Spec paths. -->
<!-- Surprising: docs/features/common-snowflake-id/orchestration/worktree-plan.md was still using obsolete eliteforge/* branch naming and a 7-column Agent Worktrees table, but the current validator enforces Lite Spec branch/worktree mirroring and an 8-column table shape. -->
