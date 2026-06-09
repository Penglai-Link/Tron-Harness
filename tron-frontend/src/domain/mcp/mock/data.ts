import type {
  ListPlatformMcpServersQuery,
  McpHealthStatus,
  McpServerTransportKind,
  PlatformMcpGatewayPayload,
  PlatformMcpServerDefinitionPayload,
  PlatformMcpServerPayload,
  PlatformMcpServerRecord,
  VerifyPlatformMcpServerDraftPayload,
  VerifyPlatformMcpServerPayload,
  WorkspaceMcpServerRecord,
} from "@/domain/mcp/model";

const baseTimestamp = Date.parse("2026-06-05T09:30:00Z");
let timestampSequence = 0;

let gatewayConfig = {
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
    transportKind: "streamable_http",
    url: "https://mcp.example.com/knowledge",
    authKind: "bearer_static",
    authPayload: { tokenRef: "MCP_KNOWLEDGE_TOKEN" },
    headers: { "X-Workspace": "TRON" },
    headersVariableRefs: { "X-Trace-Token": "MCP_TRACE_TOKEN" },
    command: "",
    args: [],
    envVariableRefs: {},
    envPassthroughRefs: [],
    workingDirectory: null,
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
    transportKind: "streamable_http",
    url: "https://mcp.example.com/memory",
    authKind: "bearer_static",
    authPayload: { tokenRef: "MCP_MEMORY_TOKEN" },
    headers: {},
    headersVariableRefs: {},
    command: "",
    args: [],
    envVariableRefs: {},
    envPassthroughRefs: [],
    workingDirectory: null,
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
    transportKind: "stdio",
    url: "",
    authKind: "none",
    authPayload: null,
    headers: {},
    headersVariableRefs: {},
    command: "openai-dev-mcp serve-ops",
    args: ["--workspace", "TRON_CORE"],
    envVariableRefs: { MCP_TOKEN: "MCP_OPS_TOKEN" },
    envPassthroughRefs: ["PATH"],
    workingDirectory: "~/code",
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
    authPayload: record.authPayload ? { ...record.authPayload } : null,
    headers: { ...record.headers },
    headersVariableRefs: { ...record.headersVariableRefs },
    args: [...record.args],
    envVariableRefs: { ...record.envVariableRefs },
    envPassthroughRefs: [...record.envPassthroughRefs],
  };
}

function matchesHealthFilter(
  record: PlatformMcpServerRecord,
  filter: ListPlatformMcpServersQuery["status"],
) {
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
  record: {
    command?: string;
    description?: string;
    name: string;
    serverCode: string;
    transportKind?: string;
    url?: string;
  },
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
    record.url,
    record.command,
    record.transportKind,
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

function findPlatformServer(serverCode: string) {
  return platformServers[findPlatformServerIndex(serverCode)];
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

function trimOrNull(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function normalizeStringMap(value: Record<string, string>) {
  return Object.fromEntries(
    Object.entries(value)
      .map(([key, entryValue]) => [key.trim(), String(entryValue ?? "").trim()] as const)
      .filter(([key, entryValue]) => key.length > 0 && entryValue.length > 0),
  );
}

function normalizeStringList(values: string[]) {
  return Array.from(
    new Set(
      values
        .map((value) => value.trim())
        .filter((value) => value.length > 0),
    ),
  );
}

function normalizeDefinitionPayload(payload: PlatformMcpServerDefinitionPayload) {
  const transportKind = payload.transportKind ?? "streamable_http";
  if (transportKind === "stdio") {
    const command = payload.command.trim();
    if (!command) {
      throw new Error("Command is required for stdio MCP servers.");
    }
    if (payload.url.trim() || payload.authKind !== "none" || payload.authPayload) {
      throw new Error("stdio MCP servers cannot define URL or auth payload.");
    }
    if (Object.keys(payload.headers).length > 0 || Object.keys(payload.headersVariableRefs).length > 0) {
      throw new Error("stdio MCP servers cannot define HTTP headers.");
    }
    return {
      transportKind,
      name: payload.name.trim(),
      description: payload.description.trim(),
      url: "",
      authKind: "none" as const,
      authPayload: null,
      headers: {},
      headersVariableRefs: {},
      command,
      args: normalizeStringList(payload.args),
      envVariableRefs: normalizeStringMap(payload.envVariableRefs),
      envPassthroughRefs: normalizeStringList(payload.envPassthroughRefs),
      workingDirectory: trimOrNull(payload.workingDirectory),
    };
  }

  if (!payload.url.trim()) {
    throw new Error("URL is required for streamable HTTP MCP servers.");
  }

  return {
    transportKind,
    name: payload.name.trim(),
    description: payload.description.trim(),
    url: payload.url.trim(),
    authKind: payload.authKind,
    authPayload: payload.authKind === "bearer_static"
      ? { ...(payload.authPayload ?? {}) }
      : null,
    headers: normalizeStringMap(payload.headers),
    headersVariableRefs: normalizeStringMap(payload.headersVariableRefs),
    command: "",
    args: [],
    envVariableRefs: {},
    envPassthroughRefs: [],
    workingDirectory: null,
  };
}

function detectVerificationOutcome(record: PlatformMcpServerRecord): { status: McpHealthStatus; message: string | null } {
  const failingText = [
    record.url,
    record.command,
    record.name,
    record.serverCode,
  ].join(" ").toLowerCase();
  if (failingText.includes("broken") || failingText.includes("127.0.0.1:9")) {
    return {
      status: "UNHEALTHY",
      message: "Remote discovery reported a transient failure.",
    };
  }
  return {
    status: "HEALTHY",
    message: null,
  };
}

function applyVerification(record: PlatformMcpServerRecord, persist: boolean) {
  const outcome = detectVerificationOutcome(record);
  const timestamp = nextTimestamp();
  const next: PlatformMcpServerRecord = {
    ...record,
    lastHealthStatus: outcome.status,
    lastHealthMessage: outcome.message,
    lastHealthCheckedAt: timestamp,
    updatedAt: timestamp,
  };
  if (persist) {
    upsertPlatformServer(record.serverCode, next);
  }
  return clonePlatformServer(next);
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

export function getPlatformMcpServerMock(serverCode: string) {
  return clonePlatformServer(findPlatformServer(serverCode));
}

export function createPlatformMcpServerMock(payload: PlatformMcpServerPayload) {
  const normalized = normalizeDefinitionPayload(payload);
  assertUniqueServerCode(payload.serverCode.trim());
  const timestamp = nextTimestamp();
  const next: PlatformMcpServerRecord = {
    serverCode: payload.serverCode.trim(),
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

export function updatePlatformMcpServerMock(serverCode: string, payload: PlatformMcpServerDefinitionPayload) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const normalized = normalizeDefinitionPayload(payload);
  const next: PlatformMcpServerRecord = {
    ...current,
    ...normalized,
    serverCode: current.serverCode,
    enabled: current.enabled,
    lastHealthStatus: current.lastHealthStatus,
    lastHealthMessage: current.lastHealthMessage,
    lastHealthCheckedAt: current.lastHealthCheckedAt,
    updatedAt: nextTimestamp(),
  };
  upsertPlatformServer(serverCode, next);
  return clonePlatformServer(next);
}

export function enablePlatformMcpServerMock(serverCode: string) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const next: PlatformMcpServerRecord = {
    ...current,
    enabled: true,
    updatedAt: nextTimestamp(),
  };
  upsertPlatformServer(serverCode, next);
  return clonePlatformServer(next);
}

export function disablePlatformMcpServerMock(serverCode: string) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const next: PlatformMcpServerRecord = {
    ...current,
    enabled: false,
    updatedAt: nextTimestamp(),
  };
  upsertPlatformServer(serverCode, next);
  return clonePlatformServer(next);
}

export function verifyPlatformMcpServerDraftMock(payload: VerifyPlatformMcpServerDraftPayload) {
  const definition = normalizeDefinitionPayload(payload.definition);
  const timestamp = nextTimestamp();
  const next: PlatformMcpServerRecord = {
    serverCode: payload.definition.serverCode.trim(),
    name: definition.name,
    description: definition.description,
    transportKind: definition.transportKind,
    url: definition.url,
    authKind: definition.authKind,
    authPayload: definition.authPayload,
    headers: definition.headers,
    headersVariableRefs: definition.headersVariableRefs,
    command: definition.command,
    args: definition.args,
    envVariableRefs: definition.envVariableRefs,
    envPassthroughRefs: definition.envPassthroughRefs,
    workingDirectory: definition.workingDirectory,
    enabled: true,
    lastHealthStatus: "NEVER_CHECKED",
    lastHealthMessage: null,
    lastHealthCheckedAt: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  return applyVerification(next, false);
}

export function verifyPlatformMcpServerMock(serverCode: string, payload?: VerifyPlatformMcpServerPayload) {
  const current = platformServers[findPlatformServerIndex(serverCode)];
  const target = payload?.definition
    ? {
      ...current,
      ...normalizeDefinitionPayload(payload.definition),
      serverCode: current.serverCode,
      enabled: current.enabled,
      lastHealthStatus: current.lastHealthStatus,
      lastHealthMessage: current.lastHealthMessage,
      lastHealthCheckedAt: current.lastHealthCheckedAt,
      createdAt: current.createdAt,
      updatedAt: current.updatedAt,
    }
    : current;
  return applyVerification(target, !payload?.definition);
}

export function runPlatformMcpServerHealthCheckMock(serverCode: string) {
  return verifyPlatformMcpServerMock(serverCode);
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
