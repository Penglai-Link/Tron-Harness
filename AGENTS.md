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
<!-- Investigation note: ontology contract guard checks currently grep backend source for lowercase raw-query tokens such as `sparql`, `turtle`, and `raw_rdf`, so backend error and message naming must avoid those exact lowercase literals even when the feature spec discusses them. -->
<!-- Investigation note: as of 2026-05-29, the product PRD requires `Embedded Grafeo Adapter Foundation` in the first backend graph storage version, but the repository feature split had only completed scope, LPG contract, and ontology contract until a new `docs/features/embedded-grafeo-adapter-foundation/` workspace was added. -->
<!-- Investigation note: embedded Grafeo schema and named graph identifiers should not use raw project codes like `knowledge-graph` directly in GQL DDL or session commands; hyphenated names can be misparsed, so the adapter must encode storage identifiers before `CREATE SCHEMA` and `USE GRAPH`. -->
<!-- Investigation note: as of 2026-05-30, system-agent models are sourced from CLI scan results and cached in process memory only; they are returned by system-agent APIs but are not persisted as managed model entities in model catalog storage. -->
<!-- Investigation note: Playwright frontend tests that rely on mock auth must set `VITE_TRON_AUTH_DATA_SOURCE=mock` explicitly; inheriting a host `VITE_TRON_AUTH_DATA_SOURCE=api` causes hidden 401 boot failures and blank `/chats` rendering despite `VITE_TRON_DATA_SOURCE=mock`. -->
<!-- Investigation note: chats view can still exhibit whole-panel scroll chaining in Chrome at specific viewport/layout states unless the chat route explicitly locks html/body overflow while mounted; page-level container overflow settings alone were insufficient. -->
<!-- Investigation note: event bus field-key safety checks reject keys containing sensitive tokens (for example, `token`), so AGUI metadata keys like `context_window_tokens` must use alternative names such as `context_window_limit` to avoid `RejectedInvalid { field: "attr_key" }` publish failures. -->
<!-- Investigation note: as of 2026-05-31, sampled `codex exec --json` outputs expose reasoning token counts but no stable reasoning text delta events, while `opencode run --format json` requires `--thinking` to emit reasoning text events; Thinking-card streaming depends on these CLI event shapes. -->
<!-- Investigation note: as of 2026-06-01, legacy SQLite files with pre-runtime_type `system_agent_registries` schema can cause startup `agent management bootstrap scan failed: StorageFailure`; agent management repository now needs an explicit runtime_type backfill migration path. -->
<!-- Investigation note: as of 2026-06-01, shell workspace refresh that clears `workspaceSwitcherSnapshot` before API completion can blank workspace/session pages during transient `/workspaces/switcher` failures; preserve last snapshot until fresh data arrives. -->
