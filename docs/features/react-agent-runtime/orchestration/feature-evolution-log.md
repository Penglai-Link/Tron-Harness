# Feature Evolution Log: React Agent Runtime Kernel

## Summary

- feature_slug: react-agent-runtime
- total_changes: 3
- first_evolution: 2026-05-27
- last_evolution: 2026-05-30

## Change History

### #3

- change_id: 3
- timestamp: 2026-05-30
- trigger: user_request
- change_type: requirement_expansion
- gate_reopened: Phase 4A
- diff_summary: |
    What changed vs before:
    - Runtime dispatch exposed system CLI execution but had no explicit backend contract for system-agent-owned model catalogs.
    + Agent runtime management now returns system-agent model catalogs from backend records, loads them from real CLI commands during startup scan, refreshes them on `/api/v1/agents/registry/system/rescan`, and keeps the catalog in runtime memory for chat/runtime selection linkage.
- downstream_impact: feature-spec,acceptance,test-plan,tasks,worktree-plan,integration-log,reports,agent-management-runtime
- old_evidence_status: superseded
- user_approval: approved_with_defaults

### #2

- change_id: 2
- timestamp: 2026-05-30
- trigger: user_request
- change_type: requirement_expansion
- gate_reopened: Phase 4A
- diff_summary: |
    What changed vs before:
    - Runtime dispatch was in-process ReAct only.
    + Runtime dispatch now supports `main-agent` in-process plus workspace-resolved system CLI adapters (`codex`, `opencode`) with unified output/token-usage mapping.
- downstream_impact: feature-spec,acceptance,test-plan,tasks,worktree-plan,integration-log,reports,agent-invocation-runtime
- old_evidence_status: superseded
- user_approval: approved_with_defaults

### #1

- change_id: 1
- timestamp: 2026-05-27
- trigger: user_request
- change_type: requirement_expansion
- gate_reopened: Phase 3
- diff_summary: |
    What changed vs before:
    - Repository lacked a unified backend agent runtime kernel.
    + Added foundational in-process ReAct runtime kernel with frozen context/model/tool contracts and metadata-only events.
- downstream_impact: feature-spec,acceptance,test-plan,tasks,worktree-plan,integration-log,reports,backend-agent-domain
- old_evidence_status: replaced
- user_approval: approved_with_defaults

## Downstream Artifact Migration

- Runtime extension evidence reuses chat API integration tests for system CLI dispatch verification until a dedicated runtime-only invocation test suite is extracted.

## Residual Risks

- System CLI runtime execution is stateless per request in this iteration; external session continuation mapping is still pending.
