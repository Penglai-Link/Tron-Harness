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

<!-- Review note: `tron-frontend/src/shared/ui/DataTable.tsx` silently renders a disabled fallback paginator when callers omit `pagination`, and multiple pages currently rely on that fallback instead of wiring live pagination state. -->
<!-- Review note: frontend i18n currently ships only `en-US` resources even though project docs require both `en-US` and `zh-CN` from the first production-ready implementation. -->
<!-- Review note: `tron-frontend/src/domain/variable/pages/VariableDetailsPage.tsx` passes `routeVariant="details"`, but `VariableManagementPage` currently does not consume that prop. -->
<!-- Review note: frontend mock data mode does not cover auth; direct protected-route visual checks still proxy `/api/v1/auth/*` to `127.0.0.1:8080`, so in-app browser verification needs a backend or explicit auth interception. -->
