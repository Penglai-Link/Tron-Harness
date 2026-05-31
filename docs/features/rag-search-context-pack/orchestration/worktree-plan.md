# Worktree Plan: RAG 后端底座 - 检索编排与 Context Pack

## Main Agent

- feature_slug: rag-search-context-pack
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/langtaoxin/rag-search-context-pack/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-search-context-pack/base
- status: ready_for_dispatch

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/langtaoxin/rag-search-context-pack/base
- note: 规格与任务文档在 lite-spec base worktree 中生成；下一步按 hybrid 默认派发一个 backend worker。

## Multi-Agent Base Branch

- branch: feature/langtaoxin/rag-search-context-pack/base
- base_ref: master
- merge_strategy: worker branches merge back into the base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | search-context-pack | pending | pending | AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 | tdd | feature/langtaoxin/rag-search-context-pack/backend/search-context-pack | /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-search-context-pack/backend/search-context-pack | feature/langtaoxin/rag-search-context-pack/base | planned | worker |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RSC-01, AC-RSC-02, AC-RSC-03, AC-RSC-04, AC-RSC-05, AC-RSC-06, AC-RSC-07, AC-RSC-08 | spawn | backend-worker | search API、Retrieval Profile、routes、repository 读取、hydrate 和 context assembly 共享同一后端检索面，派发一个 bounded backend worker 执行 RED -> GREEN -> VERIFY | backend/search-context-pack | ready |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment resolves to `langtaoxin`.
- Worker branch paths must mirror the branch tree under `/Users/langtaoxin/.codex/worktrees/tron`.
- Worker branches and worktrees must be created from `feature/langtaoxin/rag-search-context-pack/base`.

## Merge Strategy

- 优先派发一个 backend worker 覆盖 AC-RSC-01 到 AC-RSC-08，因为 search API、Profile、route、repository read 和 context assembly 共享同一后端检索面。
- 实现完成后运行 `make be-test`，再运行 `make be-check` 和 docs final validation。

## Conflict Rules

- Main Agent owns all conflict resolution.
- 如果 RAG ingest、HTTP route、SQLite repository 或 LLM runtime 在外部提交中变化，先重新读取最新实现，再调整本阶段实现。
- Do not discard user or worker changes without recording the reason in the integration log.

## Cleanup Plan

- Keep the base branch until final evidence passes.
- Remove worker worktrees and delete merged worker branches after final acceptance.
- Merge the base branch back to master only after implementation evidence passes.

## Open Decisions

- None.
