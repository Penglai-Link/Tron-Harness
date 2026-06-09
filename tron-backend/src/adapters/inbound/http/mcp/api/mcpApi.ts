import { requestApi } from "@/shared/api/httpClient";

import type {
  ListPlatformMcpServersQuery,
  McpAuthKind,
  McpHealthStatus,
  PlatformMcpGatewayConfig,
  PlatformMcpGatewayPayload,
  PlatformMcpServerPayload,
  PlatformMcpServerRecord,
  WorkspaceMcpServerRecord,
} from "@/domain/mcp/model";

export const mcpApiEndpoints = {
  getPlatformGateway: "GET /api/v1/mcp/gateway",
  updatePlatformGateway: "PATCH /api/v1/mcp/gateway",
  listPlatformServers: "GET /api/v1/mcp/servers",
  getPlatformServer: "GET /api/v1/mcp/servers/{serverCode}",
  createPlatformServer: "POST /api/v1/mcp/servers",
  updatePlatformServer: "PATCH /api/v1/mcp/servers/{serverCode}",
  enablePlatformServer: "POST /api/v1/mcp/servers/{serverCode}/enable",
  disablePlatformServer: "POST /api/v1/mcp/servers/{serverCode}/disable",
  healthPlatformServer: "POST /api/v1/mcp/servers/{serverCode}/health",
  deletePlatformServer: "DELETE /api/v1/mcp/servers/{serverCode}",
  listWorkspaceServers: "GET /api/v1/mcp/workspaces/{workspaceCode}/servers",
  enableWorkspaceServer: "POST /api/v1/mcp/workspaces/{workspaceCode}/servers/{serverCode}/enable",
  disableWorkspaceServer: "POST /api/v1/mcp/workspaces/{workspaceCode}/servers/{serverCode}/disable",
} as const;

interface PlatformMcpGatewayApi {
  gatewayEnabled: boolean;
  variablesEndpointEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

interface PlatformMcpServerRecordApi {
  serverCode: string;
  name: string;
  description?: string | null;
  url: string;
  authKind: McpAuthKind | Uppercase<McpAuthKind>;
  authPayload?: Record<string, string> | null;
  headers?: Record<string, string> | null;
  enabled: boolean;
  lastHealthStatus?: McpHealthStatus | Lowercase<McpHealthStatus> | null;
  lastHealthMessage?: string | null;
  lastHealthCheckedAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

interface WorkspaceMcpServerRecordApi {
  serverCode: string;
  name: string;
  description?: string | null;
  enabled: boolean;
  platformEnabled: boolean;
  effectiveEnabled: boolean;
  source?: "platform" | "PLATFORM";
  workspaceOverrideDisabled: boolean;
  lastHealthStatus?: McpHealthStatus | Lowercase<McpHealthStatus> | null;
  lastHealthMessage?: string | null;
  lastHealthCheckedAt?: string | null;
  updatedAt: string;
}

function normalizeHealthStatus(status: PlatformMcpServerRecordApi["lastHealthStatus"]): McpHealthStatus {
  switch (status) {
    case "healthy":
    case "HEALTHY":
      return "HEALTHY";
    case "unhealthy":
    case "UNHEALTHY":
      return "UNHEALTHY";
    default:
      return "NEVER_CHECKED";
  }
}

function normalizeAuthKind(kind: PlatformMcpServerRecordApi["authKind"]): McpAuthKind {
  switch (kind) {
    case "BEARER_STATIC":
    case "bearer_static":
      return "bearer_static";
    default:
      return "none";
  }
}

function normalizeRecordMap(value: Record<string, string> | null | undefined) {
  if (!value) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value).filter(([key]) => key.trim().length > 0),
  );
}

function normalizePlatformGateway(record: PlatformMcpGatewayApi): PlatformMcpGatewayConfig {
  return {
    gatewayEnabled: record.gatewayEnabled,
    variablesEndpointEnabled: record.variablesEndpointEnabled,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
  };
}

function normalizePlatformServer(record: PlatformMcpServerRecordApi): PlatformMcpServerRecord {
  return {
    serverCode: record.serverCode,
    name: record.name,
    description: record.description?.trim() ?? "",
    url: record.url,
    authKind: normalizeAuthKind(record.authKind),
    authPayload: normalizeRecordMap(record.authPayload),
    headers: normalizeRecordMap(record.headers),
    enabled: record.enabled,
    lastHealthStatus: normalizeHealthStatus(record.lastHealthStatus),
    lastHealthMessage: record.lastHealthMessage ?? null,
    lastHealthCheckedAt: record.lastHealthCheckedAt ?? null,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
  };
}

function normalizeWorkspaceServer(record: WorkspaceMcpServerRecordApi): WorkspaceMcpServerRecord {
  return {
    serverCode: record.serverCode,
    name: record.name,
    description: record.description?.trim() ?? "",
    enabled: record.enabled,
    platformEnabled: record.platformEnabled,
    effectiveEnabled: record.effectiveEnabled,
    source: "platform",
    workspaceOverrideDisabled: record.workspaceOverrideDisabled,
    lastHealthStatus: normalizeHealthStatus(record.lastHealthStatus),
    lastHealthMessage: record.lastHealthMessage ?? null,
    lastHealthCheckedAt: record.lastHealthCheckedAt ?? null,
    updatedAt: record.updatedAt,
  };
}

export function fetchPlatformMcpGateway(signal?: AbortSignal) {
  return requestApi<PlatformMcpGatewayApi>(mcpApiEndpoints.getPlatformGateway, { signal }).then(normalizePlatformGateway);
}

export function updatePlatformMcpGateway(payload: PlatformMcpGatewayPayload) {
  return requestApi<PlatformMcpGatewayApi>(mcpApiEndpoints.updatePlatformGateway, {
    body: payload,
  }).then(normalizePlatformGateway);
}

export function fetchPlatformMcpServers(query: ListPlatformMcpServersQuery = {}, signal?: AbortSignal) {
  return requestApi<PlatformMcpServerRecordApi[]>(mcpApiEndpoints.listPlatformServers, {
    queryParams: {
      search: query.search,
      status: query.status,
    },
    signal,
  }).then((records) => records.map(normalizePlatformServer));
}

export function createPlatformMcpServer(payload: PlatformMcpServerPayload) {
  return requestApi<PlatformMcpServerRecordApi>(mcpApiEndpoints.createPlatformServer, {
    body: payload,
  }).then(normalizePlatformServer);
}

export function updatePlatformMcpServer(serverCode: string, payload: PlatformMcpServerPayload) {
  return requestApi<PlatformMcpServerRecordApi>(mcpApiEndpoints.updatePlatformServer, {
    pathParams: { serverCode },
    body: payload,
  }).then(normalizePlatformServer);
}

export function enablePlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(mcpApiEndpoints.enablePlatformServer, {
    pathParams: { serverCode },
  }).then(normalizePlatformServer);
}

export function disablePlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(mcpApiEndpoints.disablePlatformServer, {
    pathParams: { serverCode },
  }).then(normalizePlatformServer);
}

export function healthCheckPlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(mcpApiEndpoints.healthPlatformServer, {
    pathParams: { serverCode },
  }).then(normalizePlatformServer);
}

export function deletePlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(mcpApiEndpoints.deletePlatformServer, {
    pathParams: { serverCode },
  }).then(normalizePlatformServer);
}

export function fetchWorkspaceMcpServers(workspaceCode: string, search?: string, signal?: AbortSignal) {
  return requestApi<WorkspaceMcpServerRecordApi[]>(mcpApiEndpoints.listWorkspaceServers, {
    pathParams: { workspaceCode },
    queryParams: { search },
    signal,
  }).then((records) => records.map(normalizeWorkspaceServer));
}

export function enableWorkspaceMcpServer(workspaceCode: string, serverCode: string) {
  return requestApi<WorkspaceMcpServerRecordApi>(mcpApiEndpoints.enableWorkspaceServer, {
    pathParams: { workspaceCode, serverCode },
  }).then(normalizeWorkspaceServer);
}

export function disableWorkspaceMcpServer(workspaceCode: string, serverCode: string) {
  return requestApi<WorkspaceMcpServerRecordApi>(mcpApiEndpoints.disableWorkspaceServer, {
    pathParams: { workspaceCode, serverCode },
  }).then(normalizeWorkspaceServer);
}
