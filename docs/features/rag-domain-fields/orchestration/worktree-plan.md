# Worktree Plan: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Main Agent

- feature_slug: rag-domain-fields
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/langtaoxin/rag-domain-fields/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-domain-fields/base
- status: in_progress

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/langtaoxin/rag-domain-fields/base
- note: 用户已选择 coding subagent 执行；行为改动将在 backend/domain-fields worker worktree 中完成，lead agent 负责集成与最终验证。

## Multi-Agent Base Branch

- branch: feature/langtaoxin/rag-domain-fields/base
- base_ref: master
- merge_strategy: worker branches merge back into the base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | domain-fields | 019e7989-9ab6-75c0-880d-cea8442d12b3 | Kant | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 | tdd | feature/langtaoxin/rag-domain-fields/backend/domain-fields | /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-domain-fields/backend/domain-fields | feature/langtaoxin/rag-domain-fields/base | in_progress | worker |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 | spawn | backend-worker | 用户已选择 coding subagent；数据库迁移、种子配置、过滤校验和回归测试写入面集中，适合由一个 bounded backend TDD worker 执行 | backend/domain-fields | in_progress |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment resolves to `langtaoxin`.
- Worker branch paths must mirror the branch tree under `/Users/langtaoxin/.codex/worktrees/tron`.
- Worker branches and worktrees must be created from `feature/langtaoxin/rag-domain-fields/base`.

## Merge Strategy

- backend/domain-fields worker 覆盖 AC-RDF-01 到 AC-RDF-06，因为本阶段写入面集中在 RAG schema、seed profile 和过滤校验。
- 实现完成后运行 `make be-test`，必要时再运行 `make be-check`。

## Conflict Rules

- Main Agent owns all conflict resolution.
- 如果迁移与仓储初始化逻辑同时被外部提交修改，先重新读取最新实现，再调整本阶段补充迁移。
- Do not discard user or worker changes without recording the reason in the integration log.

## Cleanup Plan

- Keep the base branch until final evidence passes.
- If worker branches are created later, remove worker worktrees and delete merged worker branches after final acceptance.
- Merge the base branch back to master only after implementation evidence passes.

## Open Decisions

- None.
