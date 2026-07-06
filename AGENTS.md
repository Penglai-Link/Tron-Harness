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

<!-- Note: `make fe-unit-test` currently fails in some chat-react tests because the test environment's `window.localStorage` is not a full Storage implementation (`clear`/`setItem` can be missing). -->
<!-- Note: `make fe-check` currently fails on an existing TypeScript error in `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx` where `Select` is passed an `onChange` prop that is not part of the component's typed API. -->
<!-- Note: Grafeo can return an `already exists` semantic error for `CREATE SCHEMA ... IF NOT EXISTS` / `CREATE GRAPH ... IF NOT EXISTS` when a new backend core opens an existing graph store, so bootstrap code must tolerate that error explicitly. -->
<!-- Note: Knowledge graph API failures can be visually confusing because the frontend falls back to mock graph data after a failed API request, so the page may still show graph content while DevTools reports the failed network response. -->
<!-- Note: The RAG ingest profile form labels graphWritePolicy as JSON, but the frontend currently submits it as a string while the backend contract accepts a JSON Value; custom graph write policy fields may therefore be ignored unless the frontend parses the JSON before submit. -->
