# API Contract: Agent Management

## API Overview

- Platform agent-registry APIs stay under the existing `/api/v1/agents/registry/*` namespace.
- Workspace-effective system-agent APIs stay under `/api/v1/agents/workspaces/{workspaceCode}`.
- Existing run APIs under `/api/v1/agents/runs*` remain separate.
- All responses keep the uniform envelope:

```json
{
  "code": "OK",
  "message": "",
  "data": {}
}
```

## Endpoints

Platform main-agent endpoints:

- `GET /api/v1/agents/registry/main-agent`: read the singleton `Main Agent` config.
- `PATCH /api/v1/agents/registry/main-agent`: save `personality` and `instructions` for the singleton `Main Agent`.

Platform system-agent endpoints:

- `GET /api/v1/agents/registry/system`: list known system-agent rows with discovery and policy state.
- `POST /api/v1/agents/registry/system/rescan`: rescan supported system agents and return the refreshed registry rows plus scan summary counts.
- `POST /api/v1/agents/registry/system/{agentCode}/enable`: set the platform baseline for one known available system agent to enabled.
- `POST /api/v1/agents/registry/system/{agentCode}/disable`: set the platform baseline for one known available system agent to disabled.

Workspace-effective system-agent endpoints:

- `GET /api/v1/agents/workspaces/{workspaceCode}/system`: list the effective workspace system-agent rows.
- `POST /api/v1/agents/workspaces/{workspaceCode}/system/{agentCode}/enable`: remove the current-workspace local disable override and return to platform behavior.
- `POST /api/v1/agents/workspaces/{workspaceCode}/system/{agentCode}/disable`: persist the current-workspace local disable override.

## Auth And Permissions

- Platform agent-management routes require authenticated platform-admin access.
- Workspace agent-management routes require authenticated access to an active workspace.
- Unavailable system-agent rows are read-only for mutation routes.

## Error Contract

- Missing or archived workspaces must return the standard error envelope.
- Unknown `agentCode` values must return not-found style error codes in the standard envelope.
- Platform enable or disable against an unavailable system agent must fail instead of silently mutating hidden state.
- Workspace enable against a platform-disabled system agent must not make the row effectively enabled.
- Main-agent patch validation must reject empty payloads that normalize to no meaningful `personality` and no meaningful `instructions` only if the backend contract requires non-empty content; otherwise empty strings are valid explicit values.

## Frontend Backend Boundary

- Platform system-agent rows must expose stable fields for `agentCode`, `name`, `description`, `source`, `status`, `enabled`, `available`, and `lastScannedAt`.
- Workspace-effective system-agent rows must expose `inheritedFrom = Platform` and must not include the platform `Main Agent`.
- Platform UI must render only `Enable` or `Disable` actions for system-agent rows and no archive action.
- Workspace UI must treat metadata as read-only and only call enable or disable actions.

## Database And Transaction Boundary

- Platform `Main Agent` config is a singleton row.
- System-agent discovery and platform policy updates must preserve known rows across rescans.
- Workspace-local disable overrides must be scoped to one workspace and one `agentCode`.
- Main-agent config save and retrieval do not require external file mutation.

## Mock Or Adapter Expectations

- Frontend mock data and smoke flow must cover enabled, disabled, and unavailable system-agent states without inventing unsupported system agents outside the bounded `codex` and `opencode` set.
- Mock workspace data must exclude `Main Agent` and expose only inherited system-agent rows.
