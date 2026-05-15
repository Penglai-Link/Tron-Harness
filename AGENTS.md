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

<!-- Surprising: Codex attached the lead agent to /Users/clouds3n/.codex/worktrees/1045/Tron, while the EliteForge Lite Spec canonical multi-agent base path for this feature would mirror feature/clouds3n/init-rust-backend/base under the worktree root. Worker worktrees should still use mirrored Lite Spec paths. -->
<!-- Surprising: docs/features/common-snowflake-id/orchestration/worktree-plan.md was still using obsolete eliteforge/* branch naming and a 7-column Agent Worktrees table, but the current validator enforces Lite Spec branch/worktree mirroring and an 8-column table shape. -->
<!-- Surprising: this session started on detached HEAD (/Users/clouds3n/.codex/worktrees/a743/Tron) while the Lite Spec integration branch feature/clouds3n/user-management-model/base was not on the canonical worktree path, so the lead had to normalize base/worker worktrees before dispatching RED execution. -->
<!-- Surprising: Vite does not load tron-frontend/.env.example by default; frontend runtime values must come from real Vite env files such as .env.local or shell variables. -->
<!-- Surprising: rtk find does not preserve standard find flags such as -path or -print; use rtk proxy find ... when exact find predicates are required. -->
<!-- Surprising: installed eliteforge-codex-superpower validator briefly failed from helper-script regressions (`acceptance_ids_from_acceptance` missing `C.` and `WorktreeNotes` typo); the project docs were valid after patching the installed helper. -->
<!-- Surprising: feature/clouds3n/shared-ui-feedback/base was forked from feature/clouds3n/user-management-model/base, so fast-forwarding shared-ui-feedback into master also brings the completed user-management-model feature history, not only shared Toast files. -->
<!-- Surprising: base already contained an older untracked-in-worktree draft of the EV-08 `operator refinement` Playwright tests, so merging the dedicated RED worker branch required conflict resolution that preferred the stricter worker version over the stale base draft. -->
<!-- Surprising: FE-GREEN-06 (`baf1265`) surfaced as an ancestor of `feature/clouds3n/user-management-model/base` without a clean lead-side merge event from `frontend/detail-create-refinement`, so EV-08 had to record a worker-isolation breach and recover by lineage audit plus focused VERIFY/REVIEW instead of a normal merge trail. -->
<!-- Surprising: TEST-FIX-02 commit `6e9fff4` landed on `feature/clouds3n/user-management-model/base` while the dedicated `test/fix-operator-refinement-create-assertions` branch pointer stayed at `47cfa62`, so the retained-smoke alignment also required lineage audit and TEST-VERIFY-07 confirmation instead of a clean merge trail. -->
<!-- Surprising: once UserDetailsPage correctly switched to `GET /api/v1/admin/users/{userCode}`, frontend detail fixtures that only mocked the list route or reset route no longer reached the detail UI; any browser test touching detail now needs both the list route and the direct detail route. -->
<!-- Surprising: a feature branch can appear aligned with `master` after a fast-forward while still carrying large uncommitted feature work in its worktree, so branch tip equality alone is not evidence that the feature has been committed. -->
<!-- Surprising: after auth lands on `master`, older feature tests that open protected admin pages or call admin APIs anonymously will fail after rebase until they bootstrap an authenticated session explicitly. -->
<!-- Surprising: the reviewed login-page OAuth2 button copy is not owned purely by frontend i18n; the live label comes from the backend auth-provider payload, so browser-visible text can stay `Development OAuth2` even after frontend layout copy changes unless the frontend normalizes the dev provider display. -->
<!-- Surprising: docs/features/auth still recorded a mirrored Lite Spec worktree on `feature/clouds3n/auth`, but the interrupted recovery resumed from the root repo on `master` with uncommitted auth changes, so the completion pass had to recreate `feature/clouds3n/auth` in place before continuing. -->
<!-- Surprising: when GitHub OAuth uses a fixed loopback callback on `127.0.0.1`, starting the authorize request from `http://localhost:<port>` stores the OAuth state cookie on `localhost`; the callback then returns to `127.0.0.1` without that cookie and fails with `AUTH_OAUTH2_STATE_INVALID`, so frontend local auth entrypoints must normalize to the callback host. -->
<!-- Surprising: local auth verification can fail for reasons unrelated to Tron when another dev server already occupies `127.0.0.1:8080`; Vite's `/api` proxy will happily forward auth traffic to that foreign process unless the Tron backend port and proxy target are realigned together. -->
<!-- Surprising: many route pages still render `TronShell` even though `AppRoutePage` already wraps resolved routes with `TronShell`; nested shells rely on `TronShellFrameContext` to collapse inner shells into inline content instead of duplicating the fixed header. -->
<!-- Surprising: the deterministic dev OAuth2 auto-provision password originally had no guaranteed digit, so the local `dev-auth-operator` subject failed `validate_initial_password` and surfaced only as `AUTH_INTERNAL_ERROR`; GitHub tests missed it because their mocked subject ids already contained digits. -->
<!-- Surprising: `make be-run` sources `tron-backend/.env` directly, so auth scope values containing spaces must be shell-quoted; an unquoted `TRON_AUTH_GITHUB_SCOPE=read:user user:email` silently truncates the variable to `read:user` and emits `user:email: command not found`. -->
