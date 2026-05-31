# API Contract: Chat Management

## API Overview

Protected Chat Management APIs own durable session and message state. They call the existing agent invocation service for execution, but frontend chat flows do not call `/api/v1/agents/runs` directly.

## Endpoints

### GET /api/v1/chats/session-groups - List Chat Session Groups

- purpose: load the left session sidebar grouped by task.
- auth: authenticated user.
- acceptance_ids: AC-CM-01
- parameter_passing: query parameters.
- request: optional `workspaceCode`.
- response: `ChatSessionGroup[]` with `taskCode`, `taskTitle`, and `sessions`.
- client_impact: replaces static mock endpoint shape.
- compatibility: existing mock-mode frontend keeps sample fallback data.

### POST /api/v1/chats/sessions - Create Chat Session

- purpose: create an empty durable session.
- auth: authenticated user.
- acceptance_ids: AC-CM-01
- parameter_passing: request body.
- request: `workspaceCode`, optional `title`, `appCode`, `taskCode`, `agentCode`.
- response: `ChatSession`.
- client_impact: used by new chat button.
- compatibility: additive endpoint.

### GET /api/v1/chats/sessions/{sessionCode}/messages - List Chat Messages

- purpose: replay persisted messages for the selected session.
- auth: authenticated user.
- acceptance_ids: AC-CM-01, AC-CM-03
- parameter_passing: path variable.
- request: `sessionCode`.
- response: `ChatMessage[]`.
- client_impact: replaces variant-based static message reads.
- compatibility: additive endpoint.

### POST /api/v1/chats/sessions/{sessionCode}/messages - Send Chat Message

- purpose: persist user message, run the agent, persist assistant final output, and return the updated transcript.
- auth: authenticated user.
- acceptance_ids: AC-CM-02, AC-CM-09
- parameter_passing: path variable and request body.
- request: `content`, `workspaceCode`, optional `appCode`, `taskCode`, `agentCode`, `model`, `limits`, `tools`.
- agent_resolution: `agentCode` in request > session `agentCode` > `main-agent`; `main-agent` uses in-process runtime, `codex`/`opencode` use system CLI runtime when workspace policy is enabled and available.
- response: `ChatSendResult` with `session`, `run`, and `messages`.
- client_impact: composer sends through Chat API.
- compatibility: additive endpoint.

### POST /api/v1/chats/sessions/{sessionCode}/messages/stream - Stream Chat Message

- purpose: persist user message, run the agent through the streaming model path, forward EventBus-backed AG-UI standard SSE events, persist assistant final output, and end with the updated transcript.
- auth: authenticated user.
- acceptance_ids: AC-CM-02, AC-CM-05, AC-CM-09.
- parameter_passing: path variable and request body.
- request: same payload as non-stream send.
- response: `text/event-stream` frames:
  - `event: RUN_STARTED`, `TEXT_MESSAGE_START`, `TEXT_MESSAGE_CONTENT`, `TEXT_MESSAGE_END`, `RUN_FINISHED` for run and assistant text lifecycle.
  - `event: REASONING_*`, `TOOL_CALL_*`, and `ACTIVITY_DELTA` when corresponding agent events are observed.
  - final `event: RUN_FINISHED`, `data: {"type":"RUN_FINISHED","result": ChatSendResult}` after persistence completes.
  - `event: RUN_ERROR`, `data: {"type":"RUN_ERROR","code":"...","message":"..."}` for invocation failures after stream setup.
- client_impact: composer uses `TEXT_MESSAGE_CONTENT` for live Markdown rendering and replaces optimistic messages with persisted messages from `CUSTOM chat.result`.
- compatibility: non-stream send remains available for clients that do not consume SSE.

## Auth And Permissions

All endpoints use existing protected-route auth. The first version validates active workspace through the existing agent invocation path on send and stores the authenticated user code on created sessions/messages. System agent execution also requires the selected workspace agent policy to be enabled and available.

## Error Contract

- missing session -> 404 / `CHAT_SESSION_NOT_FOUND`; unified error response; AC-CM-01.
- missing content -> 400 / `CHAT_MESSAGE_REQUIRED`; unified error response; AC-CM-02.
- storage failure -> 500 / `CHAT_STORAGE_FAILED`; unified error response; AC-CM-01.
- agent invocation failure -> mapped existing agent error code; unified error response; AC-CM-02.
- unavailable system agent -> 500 runtime failure with explicit disabled/unavailable reason; unified error response; AC-CM-09.

## Frontend Backend Boundary

Backend owns session/message persistence and send orchestration. Frontend owns layout, active session selection, mock-mode optimism, and polling existing AG-UI event snapshots for current-run context.

## Database And Transaction Boundary

The user message is committed before invocation. The assistant final output and session latest-run fields are committed after invocation completes. Agent invocation errors keep the user message as durable history. Session `agentCode` is persisted at session create and reused when send payload does not specify another agent.

## Mock Or Adapter Expectations

Mock mode supplies deterministic grouped sessions, messages, and AG-UI-like current-run events. Backend tests use a local mock model server for deterministic final output.
