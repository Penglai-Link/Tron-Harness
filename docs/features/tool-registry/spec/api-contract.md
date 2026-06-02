# API Contract: Tool Registry

## API Overview

All endpoints use the uniform envelope:

```json
{
  "code": "response status code or error code",
  "message": "error message",
  "data": {}
}
```

## Endpoints

- `GET /api/v1/tools/domains`
- `GET /api/v1/tools`
- `GET /api/v1/tools/{toolCode}`

The catalog is read-only in v1 and derives from backend code definitions.

## Workspace Internal-Agent Endpoints

- `GET /api/v1/agents/workspaces/{workspaceCode}/internal`
- `GET /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}`
- `PATCH /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}/policy`
- `PUT /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}/tool-assignments`
- `POST /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}/reset-policy`

## Auth And Permissions

- Tool catalog endpoints follow the same authenticated platform management conventions as other settings pages.
- Workspace internal-agent policy endpoints require an authenticated workspace-aware operator context.

## Existing Agent Management Compatibility

- Existing system-agent registry and workspace system-agent endpoints remain unchanged.
- Internal-agent APIs are parallel rather than overloading system-agent rows.

## Runtime Discovery Contract

`domain_discovery` uses runtime discovery tools rather than HTTP endpoints for expansion:

- `discovery.expand_tool_domain(domainCode)`

The discovery response must include domain metadata plus concrete assigned tools and their full JSON Schemas.

## Error Contract

Expected feature errors include:

- invalid workspace
- internal agent not found
- invalid policy combination
- tool not found in catalog
- selected tool not visible to the internal agent
- stale selected tool definition

## Frontend Backend Boundary

- Tool catalog page is read-only against backend APIs.
- Workspace `/agents` internal-agent editor reads and writes explicit policy APIs.
- Frontend mock adapters should mirror the new contract shape instead of inventing a different UI-only model.

## Database And Transaction Boundary

- Tool catalog reads are code-backed and do not persist catalog definitions.
- Workspace internal-agent policy writes persist overrides and explicit selected-tool assignments only.
- Runtime discovery expansion is run-scoped behavior, not a persisted HTTP transaction.

## Mock Or Adapter Expectations

- Frontend mock data should expose the same domain summary fields and internal-agent policy shapes as the backend contract.
- Frontend must not invent a different selected-tool payload shape from the backend assignment endpoint.
