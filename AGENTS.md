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
