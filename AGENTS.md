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

<!-- 2026-06-24: During browser testing, clicking "New session" could briefly leave the URL sessionCode ahead of the header/active chat session; verify route and active-session synchronization before relying on the URL alone. -->
<!-- Note: desktop-build runs ensure-electron before electron-builder; set build.electronDist so electron-builder reuses node_modules/electron/dist instead of downloading Electron again. -->
<!-- 2026-06-26: In tron-frontend TypeScript builds, importing `github-markdown-css` by bare package name can fail module resolution for side-effect imports; prefer `github-markdown-css/github-markdown.css`. -->
<!-- 2026-06-27: ChatManagementPage once triggered repeated React "Maximum update depth exceeded" logs because a freshly-created rightPanel ReactNode with unstable callbacks was pushed through useConsoleShellConfig into App state on every render; keep Shell config ReactNodes/callback props stable. -->
<!-- 2026-06-27: `make fe-unit-test` can fail in packages/chat-react tests when the current Node/Vitest environment lacks `window.localStorage`; the failure appears before app page tests and is unrelated to ChatManagementPage changes. -->
