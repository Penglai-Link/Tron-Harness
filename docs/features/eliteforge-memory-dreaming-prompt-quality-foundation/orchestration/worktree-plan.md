# Main Agent

feature_slug: eliteforge-memory-dreaming-prompt-quality-foundation
execution_mode: parallel
git_skill: eliteforge-git-feature-oriented-lite-spec
main_branch: feature/baiyihuan/memory/base
main_worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base
status: reopened_for_skill_quality

# Repository State

default_branch: master
current_branch: feature/baiyihuan/memory/base
current_worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base
repo_root: /home/baiyihuan/code-ai/tron

# Multi-Agent Base Branch

branch: feature/baiyihuan/memory/base
worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base
base_ref: master
status: ready

# Agent Worktrees

| git agent id | subtask name | codex agent id | codex agent name | ac_refs | tdd phase | branch | worktree path | base ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | script-contract | 019e6f29-138c-7fb2-b886-5babe0e5194d | Herschel | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-07 | tdd | feature/baiyihuan/memory/backend/script-contract | /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/backend/script-contract | feature/baiyihuan/memory/base | done | worker |
| test | quality-evaluator | 019e6f29-5948-7c23-88d8-d4b865410c1d | Russell | AC-MDPQ-04, AC-MDPQ-05 | tdd | feature/baiyihuan/memory/test/quality-evaluator | /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/test/quality-evaluator | feature/baiyihuan/memory/base | done | worker |
| docs | skill-quality-prompts | 019e71d6-3bbe-7b11-a99e-2f2eae9df34b | Kuhn | AC-MDPQ-08, AC-MDPQ-09 | tdd | feature/baiyihuan/memory/docs/skill-quality-prompts | /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/docs/skill-quality-prompts | feature/baiyihuan/memory/base | done | worker |
| backend | governance-dry-run | 019e721b-5974-7b41-b418-8c95e03e5454 | Singer | AC-MDPQ-10 | tdd | feature/baiyihuan/memory/backend/governance-dry-run | /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/backend/governance-dry-run | feature/baiyihuan/memory/base | done | worker |

# Branch And Path Rules

- Base branch follows `feature/<developer>/<taskName>/base`.
- Worker branches follow `feature/<developer>/<taskName>/<agentId>/<subtaskName>`.
- Worktree paths mirror branch paths under `/home/baiyihuan/.codex/worktrees/tron`.
- Worker branches merge back only into `feature/baiyihuan/memory/base`.

# Merge Strategy

- Merge `backend/script-contract` first to establish V2 runtime contract.
- Merge `test/quality-evaluator` second and resolve Makefile or fixture integration conflicts in base.
- Merge `docs/skill-quality-prompts` next to establish real-agent skill and prompt quality.
- Merge `backend/governance-dry-run` after prompt contracts are stable, then resolve schema and validation conflicts in base.
- Worker prompts for the reopened scope must include only the assigned AC rows plus relevant matrix row ranges from `spec/test-plan.md#matrix-coverage` and source pointers from `spec/source-coverage-audit.md`; do not paste the whole original design into worker context.
- Run full Makefile-first validation after implementation merges.
- Run the mandatory private Codex CLI skill evaluation only after implementation is ready and the user approves the exact prompt, sample scope, artifact root, redaction policy, and rubric.

# Conflict Rules

- Do not let workers revert each other's changes.
- If both workers need the same file, prefer the smaller contract-preserving edit and record the conflict in `integration-log.md`.
- Privacy controls win over convenience output.

# Cleanup Plan

- Keep worker worktrees until integration and final validation finish.
- Remove generated test artifacts before final status.
- Do not remove branches until the user confirms cleanup after the feature is complete.

# Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-07 | parallel | backend-worker | stable script contract scope with isolated runtime files | backend/script-contract | ready |
| AC-MDPQ-04, AC-MDPQ-05 | parallel | test-worker | stable evaluator scope with isolated fixture and helper files | test/quality-evaluator | ready |
| AC-MDPQ-08, AC-MDPQ-09 | spawn | docs-worker | real-agent skill guidance and prompt fixtures share files but stay isolated from script governance mechanics; matrix refs: FLOW-001..055, QUALITY-001..085, LTM-001..078, source audit, GAP | docs/skill-quality-prompts | done |
| AC-MDPQ-10 | spawn | backend-worker | governance behavior changes touch script validation and schemas, so it needs a bounded TDD worker; matrix refs: GOV-001..056 plus governance-critical QUALITY/FLOW/LTM/source-audit/GAP | backend/governance-dry-run | done |
| AC-MDPQ-11, AC-MDPQ-12 | lead | lead-reviewer | mandatory final private historical data run completed after user approval; raw private data was not delegated; committed evidence is sanitized aggregate-only | base | done |
| AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 | lead | lead-integrator | user approved all five quality-uplift points; changes intentionally span shared importer, schema, prompt, evaluator, fixture, and private-evaluation contract, so splitting would create overlapping write scopes | base | done |

# Open Decisions

- none
