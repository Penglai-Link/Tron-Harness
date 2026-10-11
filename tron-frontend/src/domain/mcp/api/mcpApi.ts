import { requestApi } from "@/shared/api/httpClient";

import type {
  ListPlatformMcpServersQuery,
  McpAuthKind,
  McpHealthStatus,
  McpServerTransportKind,
  PlatformMcpGatewayConfig,
  PlatformMcpGatewayPayload,
  PlatformMcpServerDefinitionPayload,
  PlatformMcpServerPayload,
  PlatformMcpServerRecord,
  VerifyPlatformMcpServerDraftPayload,
  VerifyPlatformMcpServerPayload,
  WorkspaceMcpServerRecord,
} from "@/domain/mcp/model";

export const mcpApiEndpoints = {
  getPlatformGateway: "GET /api/v1/mcp/gateway",
  updatePlatformGateway: "PATCH /api/v1/mcp/gateway",
  listPlatformServers: "GET /api/v1/mcp/servers",
  getPlatformServer: "GET /api/v1/mcp/servers/{serverCode}",
  createPlatformServer: "POST /api/v1/mcp/servers",
  verifyPlatformServerDraft: "POST /api/v1/mcp/servers/verify",
  updatePlatformServer: "PATCH /api/v1/mcp/servers/{serverCode}",
  verifyPlatformServer: "POST /api/v1/mcp/servers/{serverCode}/verify",
  enablePlatformServer: "POST /api/v1/mcp/servers/{serverCode}/enable",
  disablePlatformServer: "POST /api/v1/mcp/servers/{serverCode}/disable",
  healthPlatformServer: "POST /api/v1/mcp/servers/{serverCode}/health",
  deletePlatformServer: "DELETE /api/v1/mcp/servers/{serverCode}",
  listWorkspaceServers: "GET /api/v1/mcp/workspaces/{workspaceCode}/servers",
  enableWorkspaceServer:
    "POST /api/v1/mcp/workspaces/{workspaceCode}/servers/{serverCode}/enable",
  disableWorkspaceServer:
    "POST /api/v1/mcp/workspaces/{workspaceCode}/servers/{serverCode}/disable",
} as const;

interface PlatformMcpGatewayApi {
  gatewayEnabled: boolean;
  variablesEndpointEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

interface PlatformMcpServerRecordApi {
  providerPackId?: string | null;
  assignable?: boolean;
  disabledReason?: string | null;
  serverCode: string;
  name: string;
  description?: string | null;
  transportKind?: McpServerTransportKind | Uppercase<McpServerTransportKind>;
  url: string;
  authKind: McpAuthKind | Uppercase<McpAuthKind>;
  authPayload?: Record<string, string> | null;
  headers?: Record<string, string> | null;
  headersVariableRefs?: Record<string, string> | null;
  command?: string | null;
  args?: string[] | null;
  envVariableRefs?: Record<string, string> | null;
  envPassthroughRefs?: string[] | null;
  workingDirectory?: string | null;
  enabled: boolean;
  lastHealthStatus?: McpHealthStatus | Lowercase<McpHealthStatus> | null;
  lastHealthMessage?: string | null;
  lastHealthCheckedAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

interface WorkspaceMcpServerRecordApi {
  providerPackId?: string | null;
  assignable?: boolean;
  disabledReason?: string | null;
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

interface PlatformMcpDeleteResultApi {
  serverCode: string;
  deleted: boolean;
}

function normalizeHealthStatus(
  status: PlatformMcpServerRecordApi["lastHealthStatus"]
): McpHealthStatus {
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

function normalizeAuthKind(
  kind: PlatformMcpServerRecordApi["authKind"]
): McpAuthKind {
  switch (kind) {
    case "BEARER_STATIC":
    case "bearer_static":
      return "bearer_static";
    default:
      return "none";
  }
}

function normalizeTransportKind(
  kind: PlatformMcpServerRecordApi["transportKind"]
): McpServerTransportKind {
  switch (kind) {
    case "stdio":
    case "STDIO":
      return "stdio";
    default:
      return "streamable_http";
  }
}

function normalizeRecordMap(value: Record<string, string> | null | undefined) {
  if (!value) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value).filter(([key]) => key.trim().length > 0)
  );
}

function normalizeRecordList(value: string[] | null | undefined) {
  if (!value) {
    return [];
  }

  return value.map((item) => item.trim()).filter((item) => item.length > 0);
}

function normalizePlatformGateway(
  record: PlatformMcpGatewayApi
): PlatformMcpGatewayConfig {
  return {
    gatewayEnabled: record.gatewayEnabled,
    variablesEndpointEnabled: record.variablesEndpointEnabled,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
  };
}

function normalizePlatformServer(
  record: PlatformMcpServerRecordApi
): PlatformMcpServerRecord {
  return {
    providerPackId: record.providerPackId,
    assignable: record.assignable,
    disabledReason: record.disabledReason,
    serverCode: record.serverCode,
    name: record.name,
    description: record.description?.trim() ?? "",
    transportKind: normalizeTransportKind(record.transportKind),
    url: record.url,
    authKind: normalizeAuthKind(record.authKind),
    authPayload: record.authPayload
      ? normalizeRecordMap(record.authPayload)
      : null,
    headers: normalizeRecordMap(record.headers),
    headersVariableRefs: normalizeRecordMap(record.headersVariableRefs),
    command: record.command?.trim() ?? "",
    args: normalizeRecordList(record.args),
    envVariableRefs: normalizeRecordMap(record.envVariableRefs),
    envPassthroughRefs: normalizeRecordList(record.envPassthroughRefs),
    workingDirectory: record.workingDirectory?.trim()
      ? record.workingDirectory.trim()
      : null,
    enabled: record.enabled,
    lastHealthStatus: normalizeHealthStatus(record.lastHealthStatus),
    lastHealthMessage: record.lastHealthMessage ?? null,
    lastHealthCheckedAt: record.lastHealthCheckedAt ?? null,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
  };
}

function normalizeWorkspaceServer(
  record: WorkspaceMcpServerRecordApi
): WorkspaceMcpServerRecord {
  return {
    providerPackId: record.providerPackId,
    assignable: record.assignable,
    disabledReason: record.disabledReason,
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

function normalizePayload(
  payload: PlatformMcpServerDefinitionPayload | PlatformMcpServerPayload
) {
  return {
    ...payload,
    description: payload.description.trim(),
    url: payload.url.trim(),
    authPayload: payload.authPayload
      ? normalizeRecordMap(payload.authPayload)
      : null,
    headers: normalizeRecordMap(payload.headers),
    headersVariableRefs: normalizeRecordMap(payload.headersVariableRefs),
    command: payload.command.trim(),
    args: normalizeRecordList(payload.args),
    envVariableRefs: normalizeRecordMap(payload.envVariableRefs),
    envPassthroughRefs: normalizeRecordList(payload.envPassthroughRefs),
    workingDirectory: payload.workingDirectory.trim(),
  };
}

export function fetchPlatformMcpGateway(signal?: AbortSignal) {
  return requestApi<PlatformMcpGatewayApi>(mcpApiEndpoints.getPlatformGateway, {
    signal,
  }).then(normalizePlatformGateway);
}

export function updatePlatformMcpGateway(payload: PlatformMcpGatewayPayload) {
  return requestApi<PlatformMcpGatewayApi>(
    mcpApiEndpoints.updatePlatformGateway,
    {
      body: payload,
    }
  ).then(normalizePlatformGateway);
}

export function fetchPlatformMcpServers(
  query: ListPlatformMcpServersQuery = {},
  signal?: AbortSignal
) {
  return requestApi<PlatformMcpServerRecordApi[]>(
    mcpApiEndpoints.listPlatformServers,
    {
      queryParams: {
        search: query.search,
        status: query.status,
      },
      signal,
    }
  ).then((records) => records.map(normalizePlatformServer));
}

export function fetchPlatformMcpServer(
  serverCode: string,
  signal?: AbortSignal
) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.getPlatformServer,
    {
      pathParams: { serverCode },
      signal,
    }
  ).then(normalizePlatformServer);
}

export function createPlatformMcpServer(payload: PlatformMcpServerPayload) {
  const requestPayload = normalizePayload(payload);
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.createPlatformServer,
    {
      body: requestPayload,
    }
  ).then(normalizePlatformServer);
}

export function verifyPlatformMcpServerDraft(
  payload: VerifyPlatformMcpServerDraftPayload
) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.verifyPlatformServerDraft,
    {
      body: {
        workspaceCode: payload.workspaceCode,
        definition: normalizePayload(payload.definition),
      },
    }
  ).then(normalizePlatformServer);
}

export function updatePlatformMcpServer(
  serverCode: string,
  payload: PlatformMcpServerDefinitionPayload
) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.updatePlatformServer,
    {
      pathParams: { serverCode },
      body: normalizePayload(payload),
    }
  ).then(normalizePlatformServer);
}

export function verifyPlatformMcpServer(
  serverCode: string,
  payload?: VerifyPlatformMcpServerPayload
) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.verifyPlatformServer,
    {
      pathParams: { serverCode },
      body: payload?.definition
        ? {
            workspaceCode: payload.workspaceCode,
            definition: normalizePayload(payload.definition),
          }
        : {
            workspaceCode: payload?.workspaceCode,
          },
    }
  ).then(normalizePlatformServer);
}

export function enablePlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.enablePlatformServer,
    {
      pathParams: { serverCode },
    }
  ).then(normalizePlatformServer);
}

export function disablePlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.disablePlatformServer,
    {
      pathParams: { serverCode },
    }
  ).then(normalizePlatformServer);
}

export function healthCheckPlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpServerRecordApi>(
    mcpApiEndpoints.healthPlatformServer,
    {
      pathParams: { serverCode },
    }
  ).then(normalizePlatformServer);
}

export function deletePlatformMcpServer(serverCode: string) {
  return requestApi<PlatformMcpDeleteResultApi>(
    mcpApiEndpoints.deletePlatformServer,
    {
      pathParams: { serverCode },
    }
  );
}

export function fetchWorkspaceMcpServers(
  workspaceCode: string,
  search?: string,
  signal?: AbortSignal
) {
  return requestApi<WorkspaceMcpServerRecordApi[]>(
    mcpApiEndpoints.listWorkspaceServers,
    {
      pathParams: { workspaceCode },
      queryParams: { search },
      signal,
    }
  ).then((records) => records.map(normalizeWorkspaceServer));
}

export function enableWorkspaceMcpServer(
  workspaceCode: string,
  serverCode: string
) {
  return requestApi<WorkspaceMcpServerRecordApi>(
    mcpApiEndpoints.enableWorkspaceServer,
    {
      pathParams: { workspaceCode, serverCode },
    }
  ).then(normalizeWorkspaceServer);
}

export function disableWorkspaceMcpServer(
  workspaceCode: string,
  serverCode: string
) {
  return requestApi<WorkspaceMcpServerRecordApi>(
    mcpApiEndpoints.disableWorkspaceServer,
    {
      pathParams: { workspaceCode, serverCode },
    }
  ).then(normalizeWorkspaceServer);
}
