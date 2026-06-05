# API Contract: MCP Management

## API Overview

All management endpoints use the uniform envelope:

```json
{
  "code": "response status code or error code",
  "message": "error message",
  "data": {}
}
```

The MCP transport endpoint does not use this envelope. It follows MCP HTTP transport rules and exchanges MCP protocol messages instead.

## Endpoints

Management endpoints:

### Platform Gateway

- `GET /api/v1/mcp/gateway`
  - purpose: load the platform MCP gateway singleton config
- `PATCH /api/v1/mcp/gateway`
  - purpose: update `gatewayEnabled` and `variablesEndpointEnabled`

Request body:

```json
{
  "gatewayEnabled": true,
  "variablesEndpointEnabled": true
}
```

Response data:

```json
{
  "gatewayEnabled": true,
  "variablesEndpointEnabled": true,
  "createdAt": "2026-06-05T23:07:39Z",
  "updatedAt": "2026-06-05T23:07:39Z"
}
```

### Platform MCP Server Registry

- `GET /api/v1/mcp/servers`
  - purpose: list platform-owned MCP servers
  - query:
    - `search`
    - `status` optional: `enabled | disabled | unhealthy | healthy | never_checked`
- `GET /api/v1/mcp/servers/{serverCode}`
  - purpose: load one platform-owned MCP server
- `POST /api/v1/mcp/servers`
  - purpose: create one platform-owned MCP server
- `PATCH /api/v1/mcp/servers/{serverCode}`
  - purpose: update mutable metadata for one platform-owned MCP server
- `POST /api/v1/mcp/servers/{serverCode}/enable`
  - purpose: mark one platform-owned MCP server enabled
- `POST /api/v1/mcp/servers/{serverCode}/disable`
  - purpose: mark one platform-owned MCP server disabled
- `POST /api/v1/mcp/servers/{serverCode}/health`
  - purpose: run one explicit health check and persist latest health summary
- `DELETE /api/v1/mcp/servers/{serverCode}`
  - purpose: delete one platform-owned MCP server and cascade dependent overrides and selected-server assignments

Create or update request body:

```json
{
  "serverCode": "knowledge_tools",
  "name": "Knowledge Tools",
  "description": "Workspace knowledge MCP server",
  "url": "https://mcp.example.com/tools",
  "authKind": "bearer_static",
  "authPayload": {
    "tokenRef": "TOKEN_REF_PLACEHOLDER"
  },
  "headers": {
    "X-Workspace": "TRON"
  }
}
```

Registry response data item:

```json
{
  "serverCode": "knowledge_tools",
  "name": "Knowledge Tools",
  "description": "Workspace knowledge MCP server",
  "url": "https://mcp.example.com/tools",
  "authKind": "bearer_static",
  "authPayload": {
    "tokenRef": "TOKEN_REF_PLACEHOLDER"
  },
  "headers": {
    "X-Workspace": "TRON"
  },
  "enabled": true,
  "lastHealthStatus": "HEALTHY",
  "lastHealthMessage": null,
  "lastHealthCheckedAt": "2026-06-05T23:07:39Z",
  "createdAt": "2026-06-05T23:07:39Z",
  "updatedAt": "2026-06-05T23:07:39Z"
}
```

### Workspace Effective MCP Servers

- `GET /api/v1/mcp/workspaces/{workspaceCode}/servers`
  - purpose: list effective inherited MCP servers for the current workspace
  - query:
    - `search`
- `POST /api/v1/mcp/workspaces/{workspaceCode}/servers/{serverCode}/enable`
  - purpose: restore inherited enabled behavior by removing a local disable override
- `POST /api/v1/mcp/workspaces/{workspaceCode}/servers/{serverCode}/disable`
  - purpose: persist a local disable override for one inherited enabled platform server

Workspace server response data item:

```json
{
  "serverCode": "knowledge_tools",
  "name": "Knowledge Tools",
  "description": "Workspace knowledge MCP server",
  "enabled": true,
  "platformEnabled": true,
  "effectiveEnabled": true,
  "source": "platform",
  "workspaceOverrideDisabled": false,
  "lastHealthStatus": "HEALTHY",
  "lastHealthMessage": null,
  "lastHealthCheckedAt": "2026-06-05T23:07:39Z",
  "updatedAt": "2026-06-05T23:07:39Z"
}
```

### Internal-Agent MCP Policy

The existing internal-agent read endpoints remain the source of truth for the full policy record. This feature extends those payloads and adds MCP-specific write paths.

- `PATCH /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}/mcp-policy`
  - purpose: update `mcpAccessMode` for one internal agent
- `PUT /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}/mcp-server-assignments`
  - purpose: replace selected-server assignments for one internal agent

MCP policy request body:

```json
{
  "mcpAccessMode": "selected_servers"
}
```

Selected-server assignment request body:

```json
{
  "serverCodes": ["knowledge_tools", "memory_tools"]
}
```

Extended internal-agent response fields:

```json
{
  "agentCode": "main-agent",
  "mcpAccessMode": "selected_servers",
  "selectedServerCodes": ["knowledge_tools", "memory_tools"]
}
```

## MCP Transport Endpoint

- `POST /mcp/variables`
  - purpose: inbound MCP HTTP transport for the variables capability
  - request and response bodies: MCP protocol messages, not the standard management envelope

Transport rules:

- endpoint availability depends on the platform gateway singleton config
- transport auth is separate from browser CRUD auth and must follow MCP HTTP bearer-token and resource-server conventions
- this feature does not add browser-facing OAuth administration surfaces

## Auth And Permissions

- management endpoints follow the current authenticated API boundary and are protected by existing auth middleware
- management endpoints must continue forwarding locale through `Accept-Language`
- v1 does not introduce route-level `MCP` permission enforcement
- transport endpoint uses its own bearer-token and resource-server boundary rather than the browser CRUD envelope

## Error Contract

Expected management errors include:

- invalid workspace
- MCP server not found
- duplicate server code
- invalid gateway state
- invalid MCP server definition
- platform-disabled server cannot be effectively enabled in a workspace
- stale selected server assignment
- internal agent not found
- health check failed
- storage failure

Suggested error-code family:

- `MCP_GATEWAY_DISABLED`
- `MCP_SERVER_NOT_FOUND`
- `MCP_SERVER_CODE_DUPLICATE`
- `MCP_SERVER_INVALID`
- `MCP_WORKSPACE_INVALID`
- `MCP_EFFECTIVE_ENABLE_DENIED`
- `MCP_SERVER_ASSIGNMENT_STALE`
- `MCP_INTERNAL_AGENT_NOT_FOUND`
- `MCP_HEALTH_CHECK_FAILED`

## Frontend Backend Boundary

- platform settings page uses `requestApi` for authenticated and localized registry CRUD and health actions
- workspace MCP page uses `requestApi` for current-workspace effective server reads and enable or disable actions
- `/agents` keeps internal-agent reads on the existing agent-management path and adds MCP-specific writes through the parallel internal-agent API family
- frontend must not invent a different shape for selected-server assignments from the backend contract

## Runtime Boundary

- management APIs persist server definitions and policy only
- runtime discovery and tool materialization are run-scoped behaviors, not persisted HTTP transactions
- current runtime execution still requires:
  - a registered executor path for each callable MCP tool
  - a materialized tool schema in the run context
- active runs keep their snapshot even if management APIs mutate MCP state later

## Database And Transaction Boundary

- gateway writes affect only the singleton gateway config table
- platform server delete is transactional with:
  - workspace override cleanup
  - selected-server assignment cleanup
  - cache invalidation hooks after commit
- workspace enable or disable writes affect only the current workspace override row
- selected-server assignment replace writes affect only the current `(workspaceCode, agentCode)` set

## Mock Or Adapter Expectations

- frontend mock data should expose the same gateway, registry, workspace effective server, and internal-agent selected-server fields as the backend contract
- browser tests should assert settings, workspace, and internal-agent MCP flows separately
