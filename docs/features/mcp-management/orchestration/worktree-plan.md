# Worktree Plan: MCP Management

## Main Agent

- feature_slug: mcp-management
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/clouds3n/mcp-management/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/mcp-management/base
- status: active

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/mcp-management/base
- note: user-approved brainstorming direction was materialized into a new brainstorming spec and PRD on this Lite Spec base worktree before feature docs and execution planning continued.

## Multi-Agent Base Branch

- branch: feature/clouds3n/mcp-management/base
- base_ref: master
- merge_strategy: lead integrates bounded worker results into the base branch
- conflict_owner: Main Agent

## Agent Worktrees

| git agent id | subtask name | codex agent id | codex agent name | ac_refs | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | registry-runtime | pending | pending | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04 | tdd | feature/clouds3n/mcp-management/backend/registry-runtime | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/mcp-management/backend/registry-runtime | feature/clouds3n/mcp-management/base | planned |
| frontend | pages-and-browser | pending | pending | AC-MCP-02, AC-MCP-03, AC-MCP-05 | tdd | feature/clouds3n/mcp-management/frontend/pages-and-browser | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/mcp-management/frontend/pages-and-browser | feature/clouds3n/mcp-management/base | planned |
| explorer | implementation-anchors | 019e9853-0fe9-7ba2-8041-63f0f3826301 | Sagan | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04 | explore | feature/clouds3n/mcp-management/explorer/implementation-anchors | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/mcp-management/explorer/implementation-anchors | feature/clouds3n/mcp-management/base | done |
| explorer | doc-boundary-scan | 019e9853-2999-7b21-9e6d-42d4f69aa6ba | Leibniz | AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 | explore | feature/clouds3n/mcp-management/explorer/doc-boundary-scan | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/mcp-management/explorer/doc-boundary-scan | feature/clouds3n/mcp-management/base | done |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04 | spawn | backend-worker | platform registry, workspace policy, internal-agent policy extension, router wiring, runtime materialization, and backend tests share one coherent backend write scope under `tron-backend/**`; runtime_agent_type=worker | pending | planned |
| AC-MCP-02, AC-MCP-03, AC-MCP-05 | spawn | frontend-worker | settings/workspace/agents MCP UI, localized API adapters, route wiring, and browser proof stay isolated under `tron-frontend/**`; runtime_agent_type=worker | pending | planned |
| AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04 | spawn | implementation-explorer | bounded read-only scan confirmed backend/runtime anchors, auth boundary, and HTTP-profile reuse constraints before coding; runtime_agent_type=explorer | 019e9853-0fe9-7ba2-8041-63f0f3826301 | done |
| AC-MCP-01, AC-MCP-02, AC-MCP-03, AC-MCP-04, AC-MCP-05 | spawn | doc-boundary-explorer | bounded read-only scan confirmed new-feature workspace selection, dependency feature set, and current feature-doc template expectations; runtime_agent_type=explorer | 019e9853-2999-7b21-9e6d-42d4f69aa6ba | done |

## Branch And Path Rules

- Branch and worktree mirror `eliteforge-git-feature-oriented-lite-spec`.
- Any delegated worker branch or worktree must remain under this feature base.

## Merge Strategy

- Lead owns brainstorming/PRD artifacts, feature docs, integration, and final validation.
- Backend and frontend worker outputs, if used, must merge back into `feature/clouds3n/mcp-management/base` after quick review.

## Conflict Rules

- Backend worker owns backend MCP modules, route wiring, repositories, migrations, and backend/runtime tests.
- Frontend worker owns MCP pages, route wiring, API adapters, mocks, and browser tests.
- Lead owns feature docs, worktree reports, and any cross-cutting integration refinements.

## Cleanup Plan

- Keep `feature/clouds3n/mcp-management/base` active until implementation, validation, and commit handoff are complete.
- Explorer sessions can be closed after their findings are fully reflected in docs and worktree planning.

## Open Decisions

- None.
