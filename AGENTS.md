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

<!-- Project note: docs/brainstorm is ignored even though the brainstorming workflow requires committed specs there; add an individual spec with git add -f instead of changing the repository-wide ignore policy implicitly. -->
<!-- Project note: npm 11 rejects newly added nested package dependencies that use workspace:* in this workspace; use a matching semver range so npm links the local package and publishes a valid dependency. -->
<!-- Project note: vite-plugin-dts rollupTypes uses an API Extractor compiler older than this repository's TypeScript 6 and can emit an empty entry declaration; use entryRoot with module declarations instead. -->
<!-- Project note: Node 25 enables experimental Web Storage in test workers and can override jsdom localStorage with an invalid object; run Vitest with NODE_OPTIONS=--no-experimental-webstorage. -->
<!-- Project note: npm release must receive an explicit hosted registry through NPM_PUBLISH_REGISTRY; the default Nexus /repository/npm/ endpoint is a group/proxy endpoint and publishing there returns 404 or ENEEDAUTH. -->
