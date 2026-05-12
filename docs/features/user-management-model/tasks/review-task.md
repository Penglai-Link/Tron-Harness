# Review Task: User Management Model

## Execution Contract

- feature_slug: user-management-model
- status: done
- canonical_report_target: reports/review-report.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| REVIEW-01 | review | review | review-worker | DB-VERIFY-01, BE-VERIFY-01, FE-VERIFY-01, TEST-VERIFY-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/review/final-review | docs/features/user-management-model/reports/agents/final-review.md; read-only review over tron-backend and tron-frontend user-management changes | rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase final | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-06, AC-UM-07, AC-UM-08 | reports/review-report.md | done |
| REVIEW-02 | review | review | review-worker | BE-FIX-01, FE-FIX-01, BE-FIX-02 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/review/post-fix-final-review | docs/features/user-management-model/reports/agents/post-fix-final-review.md; read-only review over post-review fix changes and final docs | rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase final | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-06, AC-UM-07, AC-UM-08 | reports/review-report.md | in_progress |

## Worktree Notes

Review uses `feature/clouds3n/user-management-model/review/final-review` for initial findings and `feature/clouds3n/user-management-model/review/post-fix-final-review` for post-fix final review. Review worktrees are read-mostly except agent report updates.

## Integration Notes

Final review compares acceptance criteria, TDD evidence, implementation evidence, worktree integration evidence, and residual risk statements.

## Open Decisions

- None.
