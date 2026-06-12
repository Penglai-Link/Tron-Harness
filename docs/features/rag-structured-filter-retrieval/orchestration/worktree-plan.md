# Worktree Plan: RAG Structured Filter Retrieval

## Main Agent

- feature_slug: rag-structured-filter-retrieval
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/xinlangtao/rag-structured-filter-retrieval/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-structured-filter-retrieval/base
- status: complete

## Repository State

- default_branch: master
- selected_base_ref: feature/xinlangtao/rag
- selected_base_reason: project AGENTS notes state local `feature/xinlangtao/rag` contains the current RAG module while fetched `origin/master` may not; RAG follow-up work should use the local RAG base until remote divergence is resolved.
- original_thread_worktree: /Users/langtaoxin/cisdigital_project/tron
- original_thread_branch: feature/xinlangtao/rag
- original_thread_dirty_files: `AGENTS.md` pre-existing and left untouched.

## Multi-Agent Base Branch

- branch: feature/xinlangtao/rag-structured-filter-retrieval/base
- base_ref: feature/xinlangtao/rag
- worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-structured-filter-retrieval/base
- merge_strategy: worker branches merge back to this base branch
- conflict_owner: lead agent

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | structured-filter | 019e9167-7847-7590-8998-499cfa8c44a2 | Chandrasekhar | AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 | tdd | feature/xinlangtao/rag-structured-filter-retrieval/backend/structured-filter | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-structured-filter-retrieval/backend/structured-filter | feature/xinlangtao/rag-structured-filter-retrieval/base | done | worker |

## Branch And Path Rules

- Branches follow `eliteforge-git-feature-oriented-lite-spec`.
- Worktree paths mirror branch paths under `$HOME/.codex/worktrees/tron`.
- No worker edits the original thread worktree.

## Merge Strategy

- Worker branch merges into `feature/xinlangtao/rag-structured-filter-retrieval/base`.
- Lead inspects changed files and reruns focused validation after integration.

## Conflict Rules

- Main Agent owns all conflict resolution.
- If worker changes overlap unexpected external edits, pause integration and record the conflict before resolving.
- Do not discard user or worker changes without recording the reason.

## Cleanup Plan

- Keep worker worktree until final acceptance.
- Do not delete branches or worktrees with unintegrated changes.

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RSF-01, AC-RSF-02, AC-RSF-03, AC-RSF-04, AC-RSF-05 | spawn | tdd-slice-worker | runtime_agent_type=worker; behavior-changing rows share the same backend search pipeline files, so parallel coding would cause file conflicts; one bounded worker owns RED -> GREEN -> focused VERIFY. Read-only explorer agents ran in parallel for impact and docs scans. | backend/structured-filter | done |

## Open Decisions

- None.
