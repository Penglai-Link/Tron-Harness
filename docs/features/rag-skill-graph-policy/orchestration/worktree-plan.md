# Worktree Plan: RAG Skill Graph Policy

## Main Agent

- feature_slug: rag-skill-graph-policy
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/xinlangtao/rag-skill-graph-policy/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-policy/base
- status: done

## Repository State

- root_worktree_path: /Users/langtaoxin/cisdigital_project/tron
- default_branch: master
- base_ref: feature/xinlangtao/rag
- current_branch: feature/xinlangtao/rag-skill-graph-policy/base
- selected_git_skill_verified: true
- note: current root worktree had uncommitted prerequisite RAG graph changes; they were copied into the isolated base worktree before this slice continued.

## Multi-Agent Base Branch

- branch: feature/xinlangtao/rag-skill-graph-policy/base
- base_ref: feature/xinlangtao/rag
- merge_strategy: merge any worker branch back into this base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RSGP-01, AC-RSGP-02, AC-RSGP-04, AC-RSGP-05 | solo | lead/tdd | user approved codex-superpower execution; implementation files are tightly coupled in graph repository ingest/model/search tests, so splitting would create merge contention. | lead/backend-policy | done |
| AC-RSGP-03 | spawn | explorer/static-scan | read-only explorer runtime was useful in parallel with lead implementation. | explorer/static-scan | done |
| AC-RSGP-06, AC-RSGP-07 | solo | lead/verify | user approved final verification in this thread; verification depends on integrated branch state. | lead/final-verify | done |

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| explorer | static-scan | 019e90c6-e8e1-7cf1-9ae0-c4691c957a14 | Darwin | AC-RSGP-03 | verify | feature/xinlangtao/rag-skill-graph-policy/explorer/static-scan | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-policy/explorer/static-scan | feature/xinlangtao/rag-skill-graph-policy/base | done |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment uses `xinlangtao`, matching the active RAG branch lineage.
- Base worktree path mirrors the branch path under `/Users/langtaoxin/.codex/worktrees/tron`.
- No worker edited the root project worktree.

## Merge Strategy

- Lead created the base worktree from `feature/xinlangtao/rag`.
- Lead copied prerequisite uncommitted graph diff from the root worktree into the isolated base worktree.
- Lead completed implementation and verification in the base worktree.
- No worker branch merge was needed.

## Conflict Rules

- Main Agent owns all conflicts.
- Do not discard root worktree changes; they may be user or prerequisite changes.
- Worker output, if any, must be reviewed before merge.

## Cleanup Plan

- Keep the base worktree and branch for user review.
- Cleanup only after user confirms this branch can be merged or removed.

## Open Decisions

- None.
