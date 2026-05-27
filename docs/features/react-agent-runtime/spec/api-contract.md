# API Contract: React Agent Runtime Kernel

## API Overview

This feature defines internal Rust contracts, not HTTP endpoints. Public invocation is owned by `agent-invocation-api`.

## Endpoints

- Not applicable for this feature.

## Request And Response DTOs

- Internal request: `AgentRunRequest`.
- Internal response: `AgentRunResult`.
- Internal model request/response: normalized model input, final output, tool call, and embedding response structs.
- Internal tool request/response: `ToolRequest` and `ToolObservation`.

## Auth And Permissions

- No HTTP authentication in this feature.
- Tool authorization is limited to the frozen `EffectiveToolSet` allow list.
- Deeper role and policy checks are deferred to invocation and concrete tool features.

## Error Contract

- `AgentError::InvalidConfiguration`
- `AgentError::ModelFailure`
- `AgentError::MissingTool`
- `AgentError::UnauthorizedTool`
- `AgentError::ToolFailure`
- `AgentError::MaxStepsExceeded`
- `AgentError::EventPublishFailed`

## Frontend Backend Boundary

- No frontend boundary.
- No browser behavior is claimed by this feature.

## Database And Transaction Boundary

- No transaction boundary.
- Runtime does not query repositories and does not persist runs.

## Mock Or Adapter Expectations

- Tests may inject scripted model clients and tools.
- Production code includes a real OpenAI-compatible model client implementation and a real tool registry.

## Open Decisions

- None.
