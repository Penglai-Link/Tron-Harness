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
- The chat management page once carried an unused `../components/terminal` import even though that module does not exist; keep package-level chat UI dependencies explicit so `make fe-check` remains runnable in a clean worktree.
- The sidebar uses the persisted navigation tree whenever the API data source is active; change the built-in SQLite menu migration as well as `shellNavigation.ts` when a default sidebar hierarchy changes.
- If you encounter anything counterintuitive, surprising or confusing in this project, please append it to this file as a comment.
- `ToggleSwitch` currently applies a hard-coded `title="12"` to every rendered label; do not treat it as user-facing copy when composing layouts around that control.
- `cargo fmt` can reformat unrelated tracked files; review and retain only feature-owned changes before committing.

<!-- On macOS, electron/tests/fs-gateway.test.cjs registers os.tmpdir() paths without realpath normalization; a symlinked temporary directory can cause path-outside-root failures. Use TMPDIR=/private/tmp make desktop-test to run with a canonical temporary root. -->
<!-- Person-AI crab-serverapi owns its reqwest client and exposes no client-level default-header hook; use the pinned Git SDK and set X-Product-Code at client construction so SSE and regular SDK paths inherit it. -->
<!-- Windows PowerShell sessions may omit HOME; make be-dev falls back to USERPROFILE so the backend resolves its default ~/.tron config and storage paths correctly. -->
<!-- Keep the be-dev target declaration outside the HOST_OS conditional; otherwise its POSIX recipe is discarded and dev-api waits forever for a backend that never started. -->
<!-- chat_ask_user answers must remain pending until the previous run releases the session; persist a readable answer summary as the continuation's user transcript so React short-term memory can consume it. -->
<!-- ChatManagementPage uses @tron/chat-internal-adapter instead of the shared HTTP client; pass the current Accept-Language through adapter getHeaders so JSON, SSE, and attachment requests stay localized. -->
<!-- chat_ask_user is injected directly by run_context_builder rather than the regular catalog definitions; construct its EffectiveTool with the run locale and materialize its schema i18n keys. -->
<!-- Build Accept-Language from the visible document locale before in-memory i18n or localStorage; HMR can temporarily retain multiple i18n instances while the page language remains authoritative. Keep ordinary API requests, service queries, and chat SSE on the same header helper. -->
<!-- OpenAI Chat Completions has no standard reasoning-summary field. Preserve explicit compatible-provider summary extensions, and when only raw reasoning_content is available, use its latest complete sentence as the user-facing fallback summary; Responses reasoning.summary remains the native OpenAI source. -->
<!-- Application default_branch is configuration, not the checked-out Git branch. Resolve project execution scopes for ordinary messages and goal continuations from the same directory inspector; preserve them in the mandatory short-term-memory prompt prefix. A missing configured cwd must fail instead of returning None and falling back to the backend process directory. -->

<!-- Manual terminal PTYs are socket-owned and load ShellIntegration. Agent terminal tools use a backend-owned execution port with the same portable-pty engine, without that AI integration; do not bind their lifetime to a frontend socket. -->
