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
<!-- `make dev-api` must depend on `install`; otherwise a fresh checkout starts the backend but the frontend exits with `vite: command not found`. -->
<!-- The `/chats` transcript is rendered by `packages/chat-react/src/TronChatPanel.tsx`; the local `ChatBubble` in `ChatManagementPage.tsx` is not used by that route. -->
<!-- A chat SSE disconnect is only an observer detach, not a Stop; aborting the runtime on disconnect can leave `agent_session_run_leases` stale when the async future never records a terminal event. -->
<!-- Live steer only applies to an active runtime that declares `supports_steer`; never emulate steer for CodexExec or OpenCodeRun by starting a concurrent run. -->
<!-- The current `make be-test` baseline passes `makefile_contract` but still fails three `rust_coding_spec_architecture` checks for `resources/mod.rs`, the `application/system_config.rs` facade size, and oversized resource/tool implementation files. -->
<!-- Construct the chat SSE lifecycle guard before the lazy response body so dropping an unpolled body still deactivates its observer without stopping the run. -->
<!-- Vue `ref` proxies a plain observer record, so identity-based SSE cleanup guards must store that record in `shallowRef`; otherwise `ref.value === originalRecord` is false and `sending` can remain stuck. -->
<!-- Chat reserves the shared active-run slot before persisting its USER message; the reservation must remain steer-ineligible until RunExecutor atomically activates it. -->
<!-- A detached Vue stream may still invoke callbacks or resolve normally; every callback and post-await commit must verify the exact observer and active session. Stop consumes the already-admitted active queue item and pauses the remaining FIFO; blindly restoring it duplicates USER messages. -->
<!-- Registry Stop must call `AgentRuntime::cancel(runCode)` only when `supports_cancel` is true; remote cancel failure is diagnostic and must not prevent the local run from terminating as `AGENT_RUN_STOPPED`. -->
<!-- Never auto-queue a steer after Abort/network failure: the backend persists USER before returning its ACK, so an unknown delivery retried through FIFO can duplicate the message. Only explicit pre-admission `CHAT_STEER_*` rejections are safe queue fallbacks. -->
<!-- Domain discovery controls model-facing schema exposure only; execution authorization must use the complete resolved effective-tool set, not the progressively exposed `context.tools`. -->
<!-- A missing or unauthorized model tool call must remain blocked but recover as a failed tool observation; it must not terminate the whole agent run. -->
<!-- While a run is active, Enter must queue by default; explicit steer may redirect work but must not silently discard unresolved earlier requests. Completion merges must retain chronological message order even when a steer ACK arrived before the original run result. -->
<!-- While chat is running, keep the original single Stop control: Enter queues, Command/Ctrl+Enter (and Command/Ctrl+End compatibility) steers, and neither React nor Vue renders Queue or Steer buttons. -->
