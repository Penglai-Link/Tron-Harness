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
<!-- Investigation note: as of 2026-06-01, workspace `/agents` currently models only inherited system agents and backend tests assert `main-agent` exclusion, so internal-agent tool assignment is a feature-evolution change across API, UI, and runtime contracts rather than an additive page tweak. -->
<!-- Investigation note: as of 2026-06-02, frontend API requests did not forward the active locale via `Accept-Language`, so backend-localized catalog data silently fell back to `en-US` even when the SPA was already switched to `zh-CN`. -->
<!-- Investigation note: as of 2026-06-02, workspace agent pages with internal-agent autosave cannot share a single busy-operation string across the whole page; internal policy saves and system-agent toggles may overlap, so UI busy tracking must support multiple concurrent operation keys. -->
<!-- Investigation note: as of 2026-06-02, browser suites that care about protected-route behavior should pin both `VITE_TRON_DATA_SOURCE` and `VITE_TRON_AUTH_DATA_SOURCE` explicitly; inherited host env can silently flip a test between mock-auth direct entry and API-auth login or 401 boot failures. -->
<!-- Investigation note: as of 2026-06-02, Shell workspace selection falls back to the primary space when `/api/v1/admin/workspaces/switcher` data omits a `depth: 1` secondary root row for the active primary; Playwright suites that provide only nested nodes such as `TRON_CORE` then issue root-scoped requests or no workspace-scoped request at all. -->
<!-- Investigation note: as of 2026-06-02, feature-doc validation hard-requires newer template sections such as `Product Source Slices`, `Task Rows`, and `Evidence Rows`; older implemented feature workspaces may also need `--phase final` instead of spec-phase validation once `tasks/`, `orchestration/`, and `reports/` artifacts already exist. -->
<!-- Investigation note: as of 2026-06-02, internal agent tool observations must not be sent to model providers as raw `role: "tool"` messages unless the outbound adapter also builds provider-specific tool-call envelopes and correlation ids; OpenAI-compatible chat providers such as Qwen can reject those requests with HTTP 400, so current adapters must downgrade tool observations to a supported plain-text role such as `user`. -->
<!-- Investigation note: as of 2026-06-03, `make be-run` sources `tron-backend/.env`, but backend auth provider exposure still comes from `tron-backend/application.yaml`; toggling GitHub OAuth2 only in `.env` leaves `/api/v1/auth/providers` on internal-only until the YAML `auth.githubOauth.enabled` block is updated to match the active frontend callback URL. -->
<!-- Investigation note: as of 2026-06-04, model/provider request-level timeouts and agent-loop watchdog timeouts were removed; `httpClients.*.connectTimeoutMs` is now the only HTTP timeout knob, and it only governs connection establishment rather than end-to-end request duration. -->
<!-- Investigation note: the model contract no longer carries a `timeoutMs`/`timeout_ms` field; runtime execution now relies on the underlying HTTP client's connect timeout only, with no request-level or agent-loop watchdog timeout. -->
<!-- Investigation note: Responses API can emit multiple `function_call` output items in a single turn; the agent runtime must execute all calls in order, append every tool output back to the next input, and only then ask the model for the next round. -->
<!-- Investigation note: Responses-protocol models must serialize system guidance into top-level `instructions`, user turns into `input`, and tool results into `function_call_output` items with the model-provided `call_id`; the legacy chat-completions path keeps role/content messages and downgrades tool entries for compatibility. -->
<!-- Investigation note: as of 2026-06-04, some older `docs/features/*` workspaces such as `chat-agui-user-cards` still use a pre-current superpower template; `validate_feature_docs.py --phase final` can then fail on document-schema requirements even when the implementation and targeted browser proof are current, so full validator success may require a dedicated workspace-template migration rather than a scoped feature fix. -->
<!-- Investigation note: chat reasoning deltas may arrive as adjacent text fragments without whitespace boundaries, so user-facing reasoning transcript assembly and collapsed summary extraction must insert separators explicitly or previews collapse into unreadable joined sentences. -->
