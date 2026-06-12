# Worktree Plan

## Selected Git Spec

- spec: eliteforge-git-feature-oriented-lite-spec
- main/base branch: feature/xinlangtao/rag-frontend-release-0.1.0/base
- main/base worktree: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/base

## Isolation Notes

The base worktree already contains substantial uncommitted RAG changes that are part of the active release baseline. Creating a subagent worktree directly from branch HEAD would omit that baseline. The lead will execute in the selected base worktree and record this as a controlled isolation deviation rather than creating a misleading child branch without the current baseline.

## Dispatch Decisions

- AC-1 through AC-5: execution_owner=lead, runtime_agent_type=n/a, mode=solo, reason=current base has required uncommitted baseline and subagent worktree from HEAD would not include it safely.
- Read-only explorer: attempted after closing old agents; used only for boundary review and does not own implementation.
- AC-6 through AC-9: execution_owner=lead, runtime_agent_type=n/a, mode=solo, reason=the user requested continuing in the active base worktree, the scope is tightly coupled across `write_plan.rs` and `write_adapters.rs`, and there is no disjoint parallel write set that would reduce risk.
