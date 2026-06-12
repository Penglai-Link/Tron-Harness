# Worktree Plan: RAG Skill Graph Corpus

## Main Agent

- feature_slug: rag-skill-graph-corpus
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/xinlangtao/rag-skill-graph-corpus/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-corpus/base
- status: done

## Repository State

- root_worktree_path: /Users/langtaoxin/cisdigital_project/tron
- default_branch: master
- base_ref: feature/xinlangtao/rag
- current_branch: feature/xinlangtao/rag-skill-graph-corpus/base
- selected_git_skill_verified: true
- note: AGENTS notes say remote master may not contain the local RAG module, so this RAG follow-up uses the existing local RAG branch as base.

## Multi-Agent Base Branch

- branch: feature/xinlangtao/rag-skill-graph-corpus/base
- base_ref: feature/xinlangtao/rag
- merge_strategy: merge worker branch back into the base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RSG-01, AC-RSG-02, AC-RSG-03, AC-RSG-04, AC-RSG-05, AC-RSG-06, AC-RSG-07, AC-RSG-08 | solo | lead | worker spawn was explicitly attempted and user-approved, but the worker produced no files before shutdown; lead completed the backend work in the isolated base worktree | lead/backend-tdd | done |
| AC-RSG-09 | solo | lead | user explicitly approved lead-owned final verification and docs/report evidence; no production code split remained | lead/final-verify | done |

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | skill-graph-backend | 019e8e67-10d6-77c3-8ebd-fcb45942423d | McClintock | AC-RSG-01, AC-RSG-02, AC-RSG-03, AC-RSG-04, AC-RSG-05, AC-RSG-06, AC-RSG-07, AC-RSG-08 | tdd | feature/xinlangtao/rag-skill-graph-corpus/backend/skill-graph-backend | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-corpus/backend/skill-graph-backend | feature/xinlangtao/rag-skill-graph-corpus/base | shutdown_no_output_done |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment resolves to `xinlangtao` for this RAG line because the current local RAG branch uses that segment.
- Worker branches mirror their worktree paths under `/Users/langtaoxin/.codex/worktrees/tron`.
- No worker edits the root project worktree.

## Merge Strategy

- Lead creates the base worktree and feature docs.
- Backend worker was attempted but shut down after no file output; lead owns RED -> GREEN -> focused VERIFY in the isolated base worktree.
- Lead completed implementation without worker diff, ran Makefile-first verification, and updated evidence reports.

## Conflict Rules

- Main Agent owns all conflicts.
- Worker must stop and report if shared RAG contracts changed outside its scope.
- Do not discard user, lead, or worker changes without recording the reason in the integration log.

## Cleanup Plan

- Keep base and worker worktrees until final evidence passes.
- After final validation, preserve branches for user review unless explicitly asked to clean them up.

## Open Decisions

- None.
