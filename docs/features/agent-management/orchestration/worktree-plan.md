# Worktree Plan: Agent Management

## Main Agent

- feature_slug: agent-management
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/clouds3n/agent-management/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-management/base
- status: complete

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/agent-management/base
- note: brainstorming and PRD were already approved on this Lite Spec base worktree, so implementation continues in place.

## Multi-Agent Base Branch

- branch: feature/clouds3n/agent-management/base
- base_ref: master
- merge_strategy: lead integrates bounded worker results into the base branch
- conflict_owner: Main Agent

## Agent Worktrees

| git agent id | subtask name | codex agent id | codex agent name | ac_refs | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| frontend | agent-pages | 019e7456-e3a0-7171-a488-7acbb73f3197 | Meitner | AC-AGENT-01, AC-AGENT-03, AC-AGENT-04 | tdd | feature/clouds3n/agent-management/frontend/agent-pages | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-management/frontend/agent-pages | feature/clouds3n/agent-management/base | done |
| explorer | backend-fit-scan | 019e744e-c825-7933-ab23-c123b21d1f96 | Gauss | AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 | explore | feature/clouds3n/agent-management/explorer/backend-fit-scan | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-management/explorer/backend-fit-scan | feature/clouds3n/agent-management/base | done |
| explorer | frontend-fit-scan | 019e744e-dce0-7441-a98b-589ff120eaf8 | Lorentz | AC-AGENT-01, AC-AGENT-03, AC-AGENT-04 | explore | feature/clouds3n/agent-management/explorer/frontend-fit-scan | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-management/explorer/frontend-fit-scan | feature/clouds3n/agent-management/base | done |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 | solo | lead-agent | user approved selective multi-agent delivery, and that approval allowed a lead-owned backend slice here because runtime, persistence, router, and integration files overlap heavily enough that a second writer would have created same-file merge races | current-thread | done |
| AC-AGENT-01, AC-AGENT-03, AC-AGENT-04 | spawn | frontend-worker | platform settings, workspace page, mock wiring, and browser smoke were isolated enough to delegate after the API boundary was clear; runtime_agent_type=worker | 019e7456-e3a0-7171-a488-7acbb73f3197 | done |
| AC-AGENT-01, AC-AGENT-02, AC-AGENT-03 | spawn | backend-explorer | bounded read-only scan to confirm concrete backend file targets before coding; runtime_agent_type=explorer | 019e744e-c825-7933-ab23-c123b21d1f96 | done |
| AC-AGENT-01, AC-AGENT-03, AC-AGENT-04 | spawn | frontend-explorer | bounded read-only scan to confirm concrete frontend file targets before coding; runtime_agent_type=explorer | 019e744e-dce0-7441-a98b-589ff120eaf8 | done |

## Branch And Path Rules

- Branch and worktree mirror `eliteforge-git-feature-oriented-lite-spec`.
- Any delegated worker branch or worktree must remain under this feature base.

## Merge Strategy

- Lead owns docs, API contract, integration, and final validation.
- Backend and frontend worker outputs, if used, must be merged back into `feature/clouds3n/agent-management/base` after quick review.

## Conflict Rules

- Backend worker owns backend agent-management modules, migrations, and backend tests.
- Frontend worker owns frontend agent-management pages, route wiring, mocks, and browser tests.
- Lead owns feature docs, worktree reports, and any cross-cutting integration refinements.

## Cleanup Plan

- Keep `feature/clouds3n/agent-management/base` active until commit or PR handoff.
- Explorer and worker sessions can be closed after their integrated evidence is preserved in this workspace.

## Open Decisions

- None.
