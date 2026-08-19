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
- If you encounter anything counterintuitive, surprising or confusing in this project, please append it to this file as a comment.
<!-- SQLite `CREATE TABLE IF NOT EXISTS` does not retrofit columns or foreign keys; resource schema upgrades need an applied-version ledger and an explicit compatibility path. -->
