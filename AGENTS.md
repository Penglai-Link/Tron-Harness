# AGENTS.md

## Instructions

- Always use `Makefile` first to bulid,test and cleanup.
- Use a different available port if the default port is already in use, and ensure the frontend and backend port configuration remains synchronized.

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

<!-- Review note: `tron-frontend/src/shared/ui/DataTable.tsx` silently renders a disabled fallback paginator when callers omit `pagination`, and multiple pages currently rely on that fallback instead of wiring live pagination state. -->
<!-- Review note: frontend i18n currently ships only `en-US` resources even though project docs require both `en-US` and `zh-CN` from the first production-ready implementation. -->
<!-- Review note: `tron-frontend/src/domain/variable/pages/VariableDetailsPage.tsx` passes `routeVariant="details"`, but `VariableManagementPage` currently does not consume that prop. -->
<!-- Review note: frontend mock data mode does not cover auth; direct protected-route visual checks still proxy `/api/v1/auth/*` to `127.0.0.1:8080`, so in-app browser verification needs a backend or explicit auth interception. -->
<!-- Review note: `useServiceQuery` callers often pass inline `apiInitialData` factories; treating those factory references as hook dependencies caused API-mode pages to enter a maximum-update-depth loop. -->
<!-- Review note: shell page config stores `rightPanel` by React node reference; page-level right panels must be memoized before `useConsoleShellConfig` registration or route renders can loop through parent shell state updates. -->
