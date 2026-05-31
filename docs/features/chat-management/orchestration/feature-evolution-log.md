# Feature Evolution Log: Chat Management

## Summary

- feature_slug: chat-management
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
    - Chat session creation supported model selection only and send runtime always used in-process main agent path.
    + Chat session creation supports `agentCode` selection (`main-agent` + workspace-enabled system agents), send runtime resolves by session/request `agentCode`, and `codex`/`opencode` execute through real CLI runtime adapters.
- downstream_impact: feature-spec,database-design,api-contract,acceptance,test-plan,tasks,worktree-plan,integration-log,reports,backend-chat-service,backend-agent-invocation,frontend-chat-page,react-agent-runtime-feature-evolution
- old_evidence_status: superseded
- user_approval: approved_with_defaults

### #2

- change_id: 2
- timestamp: 2026-05-28
- trigger: user_request
- change_type: requirement_expansion
- gate_reopened: Phase 4
- diff_summary: |
    What changed vs before:
    - Chat stream emitted chat-internal `delta` and `done` events, and the transcript rendered plain bubbles with a send success toast.
    + Chat stream must emit AG-UI standard events, the transcript must render AG-UI event cards and streaming Markdown, and send completion must not show a success toast.
- downstream_impact: acceptance,test-plan,tasks,reports,backend-stream-api,frontend-chat-page
- old_evidence_status: partially_stale
- user_approval: approved_with_defaults

### #1

- change_id: 1
- timestamp: 2026-05-27
- trigger: user_request
- change_type: requirement_expansion
- gate_reopened: Phase 3
- diff_summary: |
    What changed vs before:
    - Static sample chat pages with no durable backend chat boundary.
    + Durable Chat Management backend and three-column `/chats` workbench.
- downstream_impact: feature-spec,database-design,api-contract,acceptance,test-plan,tasks,worktrees,reports
- old_evidence_status: replaced
- user_approval: approved_with_defaults

## Downstream Artifact Migration

- Old sample chat routes are retained as compatibility entries but render the new Chat Management page.

## Residual Risks

- API-mode chat sends still require a model config because the existing agent invocation service requires one.
- System agent CLI execution is stateless per send in this version; runtime session continuation mapping and resume/fork lifecycle are not implemented yet.
