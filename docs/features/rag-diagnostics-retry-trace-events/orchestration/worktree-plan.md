# Worktree Plan: RAG 后端底座 - 诊断、重试、Trace 与事件

## Main Agent

- feature_slug: rag-diagnostics-retry-trace-events
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/langtaoxin/rag-diagnostics-retry-trace-events/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-diagnostics-retry-trace-events/base
- status: implementation_dispatched

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/langtaoxin/rag-diagnostics-retry-trace-events/base
- note: 规格与任务文档在 lite-spec base worktree 中生成；后端实现已派发给 backend worker。

## Multi-Agent Base Branch

- branch: feature/langtaoxin/rag-diagnostics-retry-trace-events/base
- base_ref: master
- merge_strategy: worker branches merge back into the base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | diagnostics-retry-trace-events | 019e7c00-aa8e-7be3-bef7-603a11ab5619 | Helmholtz | AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 | tdd | feature/langtaoxin/rag-diagnostics-retry-trace-events/backend/diagnostics-retry-trace-events | /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-diagnostics-retry-trace-events/backend/diagnostics-retry-trace-events | feature/langtaoxin/rag-diagnostics-retry-trace-events/base | in_progress | worker |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RDT-01, AC-RDT-02, AC-RDT-03, AC-RDT-04, AC-RDT-05, AC-RDT-06, AC-RDT-07, AC-RDT-08 | spawn | backend-worker | debug API、retry API、trace persistence、EventBus wiring 和 boundary tests 共享同一后端诊断面，派发一个 bounded backend worker 执行 RED -> GREEN -> VERIFY | backend/diagnostics-retry-trace-events | in_progress |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment resolves to `langtaoxin`.
- Worker branch paths must mirror the branch tree under `/Users/langtaoxin/.codex/worktrees/tron`.
- Worker branches and worktrees must be created from `feature/langtaoxin/rag-diagnostics-retry-trace-events/base`.

## Merge Strategy

- 优先派发一个 backend worker 覆盖 AC-RDT-01 到 AC-RDT-08，因为 debug/retry/trace/event 共享同一 RAG 诊断数据面。
- 实现完成后运行 `make be-test`，再运行 `make be-check` 和 docs final validation。

## Conflict Rules

- Main Agent owns all conflict resolution.
- 如果 RAG ingest/search、HTTP route、SQLite repository、LLM runtime 或 EventBus 在外部提交中变化，先重新读取最新实现，再调整本阶段实现。
- Do not discard user or worker changes without recording the reason in the integration log.

## Cleanup Plan

- Keep the base branch until final evidence passes.
- Remove worker worktrees and delete merged worker branches after final acceptance.
- Merge the base branch back to master only after implementation evidence passes.

## Open Decisions

- None.
