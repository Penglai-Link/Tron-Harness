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
<!-- Review note: frontend OAuth2 login normalizes the loopback host to `127.0.0.1`; browser checks must stay on that host after callback because switching back to `localhost` loses auth cookies. -->
<!-- Review note: `make fe-clean` removes `tron-frontend/vite.config.d.ts`; a subsequent `make fe-check` can fail with TS6305 until `tsc -b tsconfig.node.json --force` regenerates the node project outputs. -->
<!-- Review note: running Playwright against the whole `tron-frontend/tests` directory can stay silent for minutes; explicit spec-file groups give reliable progress output and still cover all specs. -->
<!-- Review note: `cargo fmt --manifest-path tron-backend/Cargo.toml --check` currently reports pre-existing formatting drift in `tron-backend/tests/chat_management_api.rs`; avoid broad formatting unless that churn is intentional. -->
<!-- Review note: Vite keeps the proxy target from its start environment; if chat stream returns 404 while backend route exists, start a fresh frontend port with TRON_FRONTEND_API_PROXY_TARGET pointing at the active backend. -->
