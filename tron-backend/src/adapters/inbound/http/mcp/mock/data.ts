import type {
  ListPlatformMcpServersQuery,
  McpHealthStatus,
  PlatformMcpGatewayConfig,
  PlatformMcpGatewayPayload,
  PlatformMcpServerPayload,
  PlatformMcpServerRecord,
  WorkspaceMcpServerRecord,
} from "@/domain/mcp/model";

const baseTimestamp = Date.parse("2026-06-05T09:30:00Z");
let timestampSequence = 0;

let gatewayConfig: PlatformMcpGatewayConfig = {
  gatewayEnabled: true,
  variablesEndpointEnabled: true,
  createdAt: "2026-06-05T09:30:00Z",
  updatedAt: "2026-06-05T09:30:00Z",
};

let platformServers: PlatformMcpServerRecord[] = [
  {
    serverCode: "knowledge_tools",
    name: "Knowledge Tools",
    description: "Workspace knowledge retrieval and corpus lookup MCP server.",
    url: "https://mcp.example.com/knowledge",
    authKind: "bearer_static",
    authPayload: { tokenRef: "MCP_KNOWLEDGE_TOKEN" },
    headers: { "X-Workspace": "TRON" },
    enabled: true,
    lastHealthStatus: "HEALTHY",
    lastHealthMessage: null,
    lastHealthCheckedAt: "2026-06-05T10:00:00Z",
    createdAt: "2026-06-05T09:30:00Z",
    updatedAt: "2026-06-05T10:00:00Z",
  },
  {
    serverCode: "memory_tools",
    name: "Memory Tools",
    description: "Memory fragment lookup and summarization MCP server.",
    url: "https://mcp.example.com/memory",
    authKind: "bearer_static",
    authPayload: { tokenRef: "MCP_MEMORY_TOKEN" },
    headers: {},
    enabled: true,
    lastHealthStatus: "UNHEALTHY",
    lastHealthMessage: "Remote discovery timed out during the last check.",
    lastHealthCheckedAt: "2026-06-05T10:05:00Z",
    createdAt: "2026-06-05T09:35:00Z",
    updatedAt: "2026-06-05T10:05:00Z",
  },
  {
    serverCode: "ops_tools",
    name: "Ops Tools",
    description: "Operational maintenance MCP server kept disabled until rollout is complete.",
    url: "https://mcp.example.com/ops",
    authKind: "none",
    authPayload: {},
    headers: {},
    enabled: false,
    lastHealthStatus: "NEVER_CHECKED",
    lastHealthMessage: null,
    lastHealthCheckedAt: null,
    createdAt: "2026-06-05T09:40:00Z",
    updatedAt: "2026-06-05T09:40:00Z",
  },
];

let workspaceDisabledOverrides: Record<string, string[]> = {
  TRON_CORE: [],
};

function nextTimestamp() {
  const timestamp = new Date(baseTimestamp + timestampSequence * 60_000).toISOString();
  timestampSequence += 1;
  return timestamp;
}

function cloneGatewayConfig() {
  return { ...gatewayConfig };
}

function clonePlatformServer(record: PlatformMcpServerRecord): PlatformMcpServerRecord {
  return {
    ...record,
    authPayload: { ...record.authPayload },
    headers: { ...record.headers },
  };
}

function matchesHealthFilter(record: PlatformMcpServerRecord, filter: ListPlatformMcpServersQuery["status"]) {
  switch (filter) {
    case "enabled":
      return record.enabled;
    case "disabled":
      return !record.enabled;
    case "healthy":
      return record.lastHealthStatus === "HEALTHY";
    case "unhealthy":
      return record.lastHealthStatus === "UNHEALTHY";
    case "never_checked":
      return record.lastHealthStatus === "NEVER_CHECKED";
    default:
      return true;
  }
}

function matchesSearch(
  record: Pick<PlatformMcpServerRecord, "serverCode" | "name" | "description"> & { url?: string },
  search?: string,
) {
  const normalizedSearch = search?.trim().toLowerCase() ?? "";
  if (!normalizedSearch) {
    return true;
  }

  return [
    record.serverCode,
    record.name,
    record.description,
    record.url ?? "",
  ].join(" ").toLowerCase().includes(normalizedSearch);
}

function assertUniqueServerCode(serverCode: string, currentServerCode?: string) {
  const duplicate = platformServers.some((record) => record.serverCode === serverCode && record.serverCode !== currentServerCode);
  if (duplicate) {
    throw new Error(`MCP server ${serverCode} already exists.`);
  }
}

function findPlatformServerIndex(serverCode: string) {
  const index = platformServers.findIndex((record) => record.serverCode === serverCode);
  if (index === -1) {
    throw new Error(`MCP server ${serverCode} was not found.`);
  }
  return index;
}

function upsertPlatformServer(serverCode: string, next: PlatformMcpServerRecord) {
  const index = findPlatformServerIndex(serverCode);
  platformServers = platformServers.map((record, recordIndex) => (recordIndex === index ? next : record));
}

function workspaceOverridesFor(workspaceCode: string) {
  if (!workspaceDisabledOverrides[workspaceCode]) {
    workspaceDisabledOverrides[workspaceCode] = [];
  }
  return workspaceDisabledOverrides[workspaceCode];
}

function setWorkspaceOverride(workspaceCode: string, serverCode: string, disabled: boolean) {
  const next = new Set(workspaceOverridesFor(workspaceCode));
  if (disabled) {
    next.add(serverCode);
  } else {
    next.delete(serverCode);
  }
  workspaceDisabledOverrides = {
    ...workspaceDisabledOverrides,
    [workspaceCode]: [...next].sort(),
  };
}

function workspaceViewFor(workspaceCode: string, record: PlatformMcpServerRecord): WorkspaceMcpServerRecord {
  const workspaceOverrideDisabled = workspaceOverridesFor(workspaceCode).includes(record.serverCode);
  const effectiveEnabled = record.enabled && !workspaceOverrideDisabled;

  return {
    serverCode: record.serverCode,
    name: record.name,
    description: record.description,
    enabled: record.enabled,
    platformEnabled: record.enabled,
    effectiveEnabled,
    source: "platform",
    workspaceOverrideDisabled,
    lastHealthStatus: record.lastHealthStatus,
    lastHealthMessage: record.lastHealthMessage,
    lastHealthCheckedAt: record.lastHealthCheckedAt,
    updatedAt: record.updatedAt,
  };
}

function normalizePayload(payload: PlatformMcpServerPayload, currentServerCode?: string) {
  const serverCode = payload.serverCode.trim();
  const name = payload.name.trim();
  const url = payload.url.trim();
  if (!serverCode || !name || !url) {
    throw new Error("Server code, name, and URL are required.");
  }
  assertUniqueServerCode(serverCode, currentServerCode);

  return {
    ...payload,
    serverCode,
    name,
    description: payload.description.trim(),
    url,
    authPayload: { ...payload.authPayload },
    headers: { ...payload.headers },
  };
}

export function getPlatformMcpGatewayMock() {
  return cloneGatewayConfig();
}

export function updatePlatformMcpGatewayMock(payload: PlatformMcpGatewayPayload) {
  gatewayConfig = {
    ...gatewayConfig,
    gatewayEnabled: payload.gatewayEnabled,
    variablesEndpointEnabled: payload.variablesEndpointEnabled,
    updatedAt: nextTimestamp(),
  };
  return cloneGatewayConfig();
}

export function listPlatformMcpServersMock(query: ListPlatformMcpServersQuery = {}) {
  return platformServers
    .filter((record) => matchesSearch(record, query.search) && matchesHealthFilter(record, query.status))
    .sort((left, right) => left.serverCode.localeCompare(right.serverCode))
    .map(clonePlatformServer);
}

export function createPlatformMcpServerMock(payload: PlatformMcpServerPayload) {
  const normalized = normalizePayload(payload);
  const timestamp = nextTimestamp();
  const next: PlatformMcpServerRecord = {
    ...normalized,
    enabled: true,
    lastHealthStatus: "NEVER_CHECKED",
    lastHealthMessage: null,
    lastHealthCheckedAt: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  platformServers = [...platformServers, next];
  return clonePlatformServer(next);
}

export function updatePlatformMcpServerMock(serverCode: string, payload: PlatformMcpServerPayload) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const normalized = normalizePayload(payload, serverCode);
  const next: PlatformMcpServerRecord = {
    ...current,
    ...normalized,
    updatedAt: nextTimestamp(),
  };
  upsertPlatformServer(serverCode, next);
  return clonePlatformServer(next);
}

export function setPlatformMcpServerEnabledMock(serverCode: string, enabled: boolean) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const next: PlatformMcpServerRecord = {
    ...current,
    enabled,
    updatedAt: nextTimestamp(),
  };
  upsertPlatformServer(serverCode, next);
  return clonePlatformServer(next);
}

export function runPlatformMcpServerHealthCheckMock(serverCode: string) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const nextStatus: McpHealthStatus =
    current.lastHealthStatus === "HEALTHY" ? "UNHEALTHY" : "HEALTHY";
  const next: PlatformMcpServerRecord = {
    ...current,
    lastHealthStatus: nextStatus,
    lastHealthMessage: nextStatus === "HEALTHY" ? null : "Remote discovery reported a transient failure.",
    lastHealthCheckedAt: nextTimestamp(),
    updatedAt: nextTimestamp(),
  };
  upsertPlatformServer(serverCode, next);
  return clonePlatformServer(next);
}

export function deletePlatformMcpServerMock(serverCode: string) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  platformServers = platformServers.filter((record) => record.serverCode !== serverCode);
  workspaceDisabledOverrides = Object.fromEntries(
    Object.entries(workspaceDisabledOverrides).map(([workspaceCode, serverCodes]) => [
      workspaceCode,
      serverCodes.filter((code) => code !== serverCode),
    ]),
  );
  return clonePlatformServer(current);
}

export function listWorkspaceMcpServersMock(workspaceCode: string, search?: string) {
  return platformServers
    .map((record) => workspaceViewFor(workspaceCode, record))
    .filter((record) => matchesSearch(record, search))
    .sort((left, right) => left.serverCode.localeCompare(right.serverCode))
    .map((record) => ({ ...record }));
}

export function setWorkspaceMcpServerEnabledMock(workspaceCode: string, serverCode: string, enabled: boolean) {
  const current = workspaceViewFor(workspaceCode, platformServers[findPlatformServerIndex(serverCode)]);
  if (enabled && !current.platformEnabled) {
    throw new Error(`Platform-disabled server ${serverCode} cannot be enabled locally.`);
  }
  setWorkspaceOverride(workspaceCode, serverCode, !enabled);
  return {
    ...workspaceViewFor(workspaceCode, platformServers[findPlatformServerIndex(serverCode)]),
  };
}

export function listEffectiveEnabledWorkspaceMcpServerCodesMock(workspaceCode: string) {
  return listWorkspaceMcpServersMock(workspaceCode)
    .filter((record) => record.effectiveEnabled)
    .map((record) => record.serverCode);
}
