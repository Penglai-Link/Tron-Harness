# Feature Spec: Chat Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The implementation request supplies the design boundary: durable sessions and messages, runs as execution history, AG-UI as current-run event context only, and a three-column management page. This feature replaces the static chat sample pages and adds a backend Chat Management layer instead of making the frontend call agent runs directly.

## Feature Identity And Context

- feature_slug: chat-management
- feature_name: Chat Management
- status: approved_with_defaults
- owner: lead
- source_request: user-provided Chat management page design summary
- main_agent: current Codex lead agent
- dependency_features: Agent Invocation API, Agent Run History, Agent AG-UI Stream Adapter, Right Context Sidebar
- dependency_contracts: uniform API envelope, protected route auth context, `agent_runs.conversation_code`, AG-UI stream replay by stream key

## Requirement

- user_story: Operators can continue historical chat sessions, send new messages, and observe the current agent run without losing durable message history.
- intent: separate chat history management from direct agent execution and prevent AG-UI replay from becoming the historical message source.
- outcome: `/chats` becomes the durable Chat Management workspace with grouped sessions, persisted transcript replay, send orchestration, session-level agent selection, and a constrained right context sidebar.

## Product Source Slices

- brainstorming_source: implementation request, `Chat 管理页设计总结`
- prd_feature_headings: approved-with-defaults from implementation request
- prd_page_headings: `页面布局`, `左栏 Session Sidebar`, `中栏 对话界面`, `右栏 Right Context Sidebar`
- prd_rule_headings: `概念边界`, `后端边界`
- prd_exception_headings: context window unavailable fallback, unassigned sessions
- prd_acceptance_headings: recommended landing order in request

## Scope Boundary

### Assumptions

- Existing synchronous agent invocation remains the first implementation path for chat sends.
- API-mode chat sends must provide a model config because the current agent runtime requires one.
- Mock-mode frontend can show deterministic AG-UI-like current-run events for visual proof.
- System agent execution in this iteration is stateless CLI invocation per send request.

### In Scope

- `chat_sessions` and `chat_messages` persistence.
- Protected `/api/v1/chats` routes for grouping sessions, creating sessions, listing messages, and sending messages.
- Chat API orchestration over existing agent invocation with `conversationCode=sessionCode`.
- Session `agentCode` persistence and chat send fallback to session-selected agent when request payload omits `agentCode`.
- Chat-layer routing that consumes runtime support for `main-agent`, `codex`, and `opencode` from `react-agent-runtime`.
- `/chats` three-column workbench and compatibility rendering for existing `/chats/8842-alpha` sample path.
- Right context sidebar limited to app/task, worktree path/branch, actually observed skills, and context-window status.

### Out Of Scope

- Workspace/app/project multi-level session trees.
- Reconstructing historical messages from AG-UI events.
- Fabricated context-window token metrics.
- Full asynchronous background run lifecycle beyond the existing invocation service.
- Cross-run or cross-request continuation mapping to external CLI sessions (resume/fork/session handoff).
- Generic passthrough of all Codex/OpenCode CLI flags from frontend payloads.

### Constraints

- Use Makefile commands first for build/test.
- Use the uniform API response envelope.
- Follow lite git branch/worktree rules.
- Preserve shell right-panel memoization to avoid route render loops.

## Feature Model

Session is the durable conversation container and owns a stable `sessionCode` plus optional `agentCode` for invocation routing. Run is a single execution under a session and uses `runCode`; the existing `agent_runs` table stores run summaries. Message is the durable historical transcript record and is never rebuilt from AG-UI. AG-UI only represents current-run event state and can supply step/status metadata and the skill names actually observed in this run.

Sending a chat message creates or uses a session, persists a user message, resolves invocation agent by payload `agentCode` or session `agentCode`, invokes the selected runtime with `conversationCode=sessionCode`, updates the latest run summary on the session, and stores the assistant final output as a message after completion.

## Page And Module Surface

- Routes: `/chats`, `/chats/8842-alpha`, `/chats/8842-alpha?variant=execution`.
- Frontend modules: `tron-frontend/src/domain/chat/**`, `tron-frontend/src/router/routes.tsx`, `tron-frontend/src/router/pageRegistry.ts`, `tron-frontend/src/app/App.tsx`.
- Backend modules: `tron-backend/src/domain/chat.rs`, `tron-backend/src/application/chat.rs`, `tron-backend/src/adapters/inbound/http/chats/**`, `tron-backend/src/adapters/outbound/persistence/sqlite/chat_repository.rs`.
- Database: `chat_sessions`, `chat_messages`, existing `agent_runs`.
- Tests: backend chat API integration test and frontend type/build validation.

The page uses the console shell content area for the left session sidebar and conversation workspace. The shell right panel is the Right Context Sidebar. Session groups are task collapsible sections with `Unassigned` fallback. Composer send updates the transcript and current run context.

## Repository Findings

- Existing chat frontend is duplicated in `ChatSessionReasoningPage.tsx` and `ChatSessionExecutionPage.tsx`.
- Existing `/chats` route redirects to `/chats/8842-alpha`; route resolution must be updated so `/chats` is first-class.
- `chat` is already a `useServiceQuery` domain and can use mock/API switching.
- Agent invocation already persists `agent_runs` and emits AG-UI events with stream key from `conversationCode`.
- AG-UI events contain metadata only; this aligns with the right context requirement but cannot supply context-window statistics yet.
- Strong feature dependencies found after scanning adjacent feature specs: Agent Invocation API, Agent Run History, Agent AG-UI Stream Adapter, Right Context Sidebar.
- Workspace system agent management already exposes enabled/available state for `codex` and `opencode`, which can be reused for chat agent selection.
- Existing invocation contract enforces model config even when execution is delegated to a CLI system agent runtime.

## Open Decisions

- Chat service capability; signal: future pages may reuse durable chat sessions; modules: backend chat API and frontend chat hooks; target: `application/chat` and `domain/chat`; recommendation: extract now as feature-local domain with reusable service boundary; decision: approved_with_defaults.
- CLI session continuity; signal: users may expect `conversation` resume across requests for Codex/OpenCode; modules: chat session model, agent runtime adapters, run history mapping; target: future evolution after current adapter foundation; recommendation: add explicit runtime session mapping contract and resume strategy as a follow-up feature; decision: pending.
