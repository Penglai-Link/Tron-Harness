# API Contract: Agent Invocation API

## API Overview

Add a protected synchronous invocation endpoint for the backend in-process ReAct runtime.

## Endpoints

- `POST /api/v1/agents/runs`
  - Auth: required bearer session.
  - Request: agent run invocation payload.
  - Response: uniform `ApiResponse<AgentRunResponse>`.

## Request And Response DTOs

Request fields:

- `prompt: string`
- `workspaceCode: string`
- `conversationCode?: string`
- `appCode?: string`
- `taskCode?: string`
- `model?: AgentModelConfigRequest`
- `limits?: AgentRunLimitsRequest`
- `tools?: AgentToolDefinitionRequest[]`

Response fields:

- `runCode: string`
- `conversationCode?: string`
- `workspaceCode: string`
- `status: "COMPLETED"`
- `finalOutput: string`
- `stepCount: number`

## Auth And Permissions

- Protected by existing `require_auth` middleware.
- Caller identity becomes `user_code` in the frozen runtime context.
- Workspace must exist and be active.
- App code, when provided, must resolve inside the workspace.

## Error Contract

- `AGENT_PROMPT_REQUIRED`
- `AGENT_WORKSPACE_REQUIRED`
- `AGENT_WORKSPACE_INVALID`
- `AGENT_MODEL_REQUIRED`
- `AGENT_RUNTIME_FAILED`
- `AGENT_MODEL_FAILED`
- `AGENT_TOOL_FAILED`
- `AGENT_MAX_STEPS_EXCEEDED`

## Frontend Backend Boundary

- No new frontend page.
- Existing frontend should continue to build and route normally.
- Browser verification covers authenticated app smoke, not a new agent page.

## Database And Transaction Boundary

- No writes in this feature.
- Reads happen before runtime execution.
- Runtime does not hold repository locks.

## Mock Or Adapter Expectations

- Tests can inject scripted model clients through application service construction or request test mode.
- Production route uses the real runtime and model client boundary.

## Open Decisions

- None.
