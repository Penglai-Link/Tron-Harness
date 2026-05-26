# Worktree Plan: Right Context Sidebar

## Main Agent

- feature_slug: right-context-sidebar
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/clouds3n/right-context-sidebar/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/base
- status: in_progress

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/right-context-sidebar/base
- note: planning and source documents were approved on master before behavior-changing work; implementation uses the Lite Spec base worktree.

## Multi-Agent Base Branch

- branch: feature/clouds3n/right-context-sidebar/base
- base_ref: master
- merge_strategy: worker branches merge back into the base branch after each bounded AC slice
- conflict_owner: Main Agent

## Agent Worktrees

| git agent id | subtask name | codex agent id | codex agent name | ac_refs | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| frontend | shell-foundation | 019e6235-9c0d-7b52-9a94-28a33838474d | Bohr | AC-RCS-01, AC-RCS-02, AC-RCS-03 | tdd | feature/clouds3n/right-context-sidebar/frontend/shell-foundation | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/frontend/shell-foundation | feature/clouds3n/right-context-sidebar/base | in_progress |
| frontend | graph-memory-migration | pending_spawn | pending_spawn | AC-RCS-04 | tdd | feature/clouds3n/right-context-sidebar/frontend/graph-memory-migration | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/frontend/graph-memory-migration | feature/clouds3n/right-context-sidebar/base | planned |
| frontend | management-migration | pending_spawn | pending_spawn | AC-RCS-05 | tdd | feature/clouds3n/right-context-sidebar/frontend/management-migration | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/frontend/management-migration | feature/clouds3n/right-context-sidebar/base | planned |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RCS-01, AC-RCS-02, AC-RCS-03 | spawn | frontend/shell-foundation | default hybrid behavior-changing shell slice; shared `Shell.tsx` work should be isolated before page migrations | frontend/shell-foundation | in_progress |
| AC-RCS-04 | spawn | frontend/graph-memory-migration | default hybrid behavior-changing migration slice after shell contract is stable | frontend/graph-memory-migration | ready_after_shell |
| AC-RCS-05 | spawn | frontend/management-migration | default hybrid behavior-changing migration slice after shell contract is stable and graph/memory pattern is known | frontend/management-migration | ready_after_graph_memory |

## Branch And Path Rules

- Branches and worktrees mirror `eliteforge-git-feature-oriented-lite-spec`.
- Main/base branch is `feature/clouds3n/right-context-sidebar/base`.
- Worker branches use `feature/clouds3n/right-context-sidebar/<agentId>/<subtaskName>`.
- Worker paths mirror branch segments under `/Users/clouds3n/.codex/worktrees/Tron`.

## Merge Strategy

- Create each worker branch from the current base branch.
- Merge `frontend/shell-foundation` first, then refresh base validation.
- Create or refresh migration worker branches only after shell foundation is integrated.
- Merge migration workers back into the base branch one at a time.

## Conflict Rules

- The lead owns conflict resolution in the base worktree.
- Do not run workers that edit `Shell.tsx` in parallel.
- If two migration workers unexpectedly edit the same page or test file, stop parallel dispatch and serialize that page group.

## Cleanup Plan

- Keep worker branches and worktrees until final acceptance.
- Remove worker worktrees only after their changes are integrated and evidence is recorded.

## Open Decisions

- None.
