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
<!-- Person-AI crab-serverapi owns its reqwest client and exposes no client-level default-header hook; vendor the SDK and set X-Product-Code at client construction so SSE and regular SDK paths inherit it. -->
<!-- Windows PowerShell sessions may omit HOME; make be-dev falls back to USERPROFILE so the backend resolves its default ~/.tron config and storage paths correctly. -->
