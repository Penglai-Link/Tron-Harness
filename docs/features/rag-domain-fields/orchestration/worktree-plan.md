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
- status: ready

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/langtaoxin/rag-domain-fields/base
- note: 本阶段规格与任务文档已在 lite-spec base worktree 中生成；行为改动需要在用户明确授权 coding subagent 或 lead solo 后开始。

## Multi-Agent Base Branch

- branch: feature/langtaoxin/rag-domain-fields/base
- base_ref: master
- merge_strategy: worker branches merge back into the base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 | blocked | lead-agent | 当前阶段已完成规格与任务切片；数据库迁移、种子配置、过滤校验和回归测试属于行为改动，需要用户明确选择 coding subagent 或 lead solo 后执行 | not_applicable | waiting_human_decision |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment resolves to `langtaoxin`.
- Worker branch paths must mirror the branch tree under `/Users/langtaoxin/.codex/worktrees/tron`.
- Worker branches and worktrees must be created from `feature/langtaoxin/rag-domain-fields/base`.

## Merge Strategy

- 如果用户授权 coding subagent，优先派发一个 backend worker 覆盖 AC-RDF-01 到 AC-RDF-06，因为本阶段写入面集中在 RAG schema、seed profile 和过滤校验。
- 如果用户授权 lead solo，则在 base worktree 中直接执行同一组 AC，任务行需要同步记录 solo rationale。
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

- 等待用户选择执行方式：coding subagent 或 lead solo。
