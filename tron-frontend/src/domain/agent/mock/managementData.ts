import type { MainAgentConfigPayload } from "@/domain/agent/api/agentManagementApi";
import type {
  ExpandedDomainPersistenceMode,
  ListToolsQuery,
  MainAgentConfig,
  McpAccessMode,
  PlatformSystemAgentRecord,
  PlatformSystemAgentRescanResult,
  PlatformToolRecord,
  ToolCatalogDomainRecord,
  ToolExposureMode,
  ToolAssignmentMode,
  WorkspaceInternalAgentMcpPolicyPayload,
  WorkspaceInternalAgentPolicyPayload,
  WorkspaceInternalAgentRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";
import { listEffectiveEnabledWorkspaceMcpServerCodesMock } from "@/domain/mcp/mock";

const scanBaseMs = Date.parse("2026-05-29T09:30:00Z");
let scanSequence = 0;

let mainAgentConfig: MainAgentConfig = {
  instructions: "Coordinate coding tasks with clear ownership boundaries, keep outputs concise, and require validation before close-out.",
  personality: "Direct, accountable, and precise. Push for concrete next steps and expose tradeoffs plainly.",
  updatedAt: "2026-05-29T09:30:00Z",
};

type SystemAgentSeed = {
  agentCode: string;
  available: boolean;
  description: string;
  models: Array<{ modelId: string; displayName: string }>;
  name: string;
  platformEnabled: boolean;
  source: string;
  lastScannedAt: string | null;
};

const systemAgentRegistry: SystemAgentSeed[] = [
  {
    agentCode: "codex",
    available: true,
    description: "OpenAI coding agent runtime used for structured feature delivery.",
    models: [
      { modelId: "gpt-5-codex", displayName: "GPT-5 Codex" },
      { modelId: "gpt-5-codex-low", displayName: "GPT-5 Codex Low" },
    ],
    name: "Codex",
    platformEnabled: true,
    source: "System Scan",
    lastScannedAt: "2026-05-29T09:30:00Z",
  },
  {
    agentCode: "opencode",
    available: true,
    description: "Local CLI coding agent kept as a platform-level fallback executor.",
    models: [
      { modelId: "openai/gpt-4.1", displayName: "OpenAI GPT-4.1" },
      { modelId: "anthropic/claude-sonnet-4", displayName: "Claude Sonnet 4" },
    ],
    name: "OpenCode",
    platformEnabled: false,
    source: "System Scan",
    lastScannedAt: "2026-05-29T09:30:00Z",
  },
];

type ToolDomainSeed = {
  domainCode: string;
  name: string;
  description: string;
  sampleTools: string[];
  hasMutationTools: boolean;
};

const toolDomainSeed: ToolDomainSeed[] = [
  {
    domainCode: "workspace",
    description: "Workspace discovery and metadata lookup for platform workspaces.",
    hasMutationTools: false,
    name: "Workspace",
    sampleTools: ["workspace.page", "workspace.get"],
  },
  {
    domainCode: "agent",
    description: "Introspection and lifecycle actions for registered internal agents and policies.",
    hasMutationTools: true,
    name: "Agent",
    sampleTools: ["agents.get", "agents.workspace.get"],
  },
  {
    domainCode: "model",
    description: "Read and inspect platform and workspace model catalogs.",
    hasMutationTools: true,
    name: "Model",
    sampleTools: ["model.page", "model.create", "model.update"],
  },
];

type ToolSeed = {
  toolCode: string;
  domainCode: string;
  name: string;
  description: string;
  operationKind: PlatformToolRecord["operationKind"];
};

const toolSeed: ToolSeed[] = [
  {
    description: "Page readable workspaces and workspace metadata.",
    domainCode: "workspace",
    name: "Page Workspaces",
    operationKind: "query",
    toolCode: "workspace.page",
  },
  {
    description: "Get workspace details and inherited policy summary.",
    domainCode: "workspace",
    name: "Get Workspace",
    operationKind: "query",
    toolCode: "workspace.get",
  },
  {
    description: "Get a single internal or system agent profile.",
    domainCode: "agent",
    name: "Get Agent",
    operationKind: "query",
    toolCode: "agents.get",
  },
  {
    description: "Get per-workspace internal-agent policy and assignment context.",
    domainCode: "agent",
    name: "Get Workspace Agent",
    operationKind: "query",
    toolCode: "agents.workspace.get",
  },
  {
    description: "Patch internal-agent policy mode and exposure behavior.",
    domainCode: "agent",
    name: "Patch Workspace Agent Policy",
    operationKind: "command",
    toolCode: "agents.workspace.patch_policy",
  },
  {
    description: "Page effective models with protocol and capability metadata.",
    domainCode: "model",
    name: "Page Models",
    operationKind: "query",
    toolCode: "model.page",
  },
  {
    description: "Create a workspace-local model definition.",
    domainCode: "model",
    name: "Create Model",
    operationKind: "command",
    toolCode: "model.create",
  },
  {
    description: "Update a workspace model definition.",
    domainCode: "model",
    name: "Update Model",
    operationKind: "command",
    toolCode: "model.update",
  },
];

type WorkspaceInternalAgentSeed = {
  agentCode: string;
  description: string;
  expandedDomainPersistenceMode: ExpandedDomainPersistenceMode;
  maxSteps: number;
  maxToolCalls: number;
  maxParallelToolCalls: number;
  mcpAccessMode: McpAccessMode;
  name: string;
  selectedServerCodes: string[];
  selectedToolCodes: string[];
  toolAssignmentMode: ToolAssignmentMode;
  toolExposureMode: ToolExposureMode;
};

const workspaceInternalAgentSeed: WorkspaceInternalAgentSeed[] = [
  {
    agentCode: "main-agent",
    description: "Primary platform internal agent used for workspace chat runtime policy.",
    expandedDomainPersistenceMode: "sticky_expanded",
    maxSteps: 100,
    maxToolCalls: 100,
    maxParallelToolCalls: 4,
    mcpAccessMode: "all_enabled_servers",
    name: "Main Agent",
    selectedServerCodes: [],
    selectedToolCodes: [],
    toolAssignmentMode: "all_platform_tools",
    toolExposureMode: "direct_list",
  },
];

let workspaceOverrides: Record<string, WorkspaceInternalAgentSeed[]> = {};
let workspaceSystemAgentOverrides: Record<string, string[]> = {};

function nextScanTimestamp() {
  const timestamp = new Date(scanBaseMs + scanSequence * 60_000).toISOString();
  scanSequence += 1;
  return timestamp;
}

function cloneMainAgentConfig(config: MainAgentConfig): MainAgentConfig {
  return {
    instructions: config.instructions,
    personality: config.personality,
    updatedAt: config.updatedAt,
  };
}

function cloneSystemAgentRecord(record: PlatformSystemAgentRecord | WorkspaceSystemAgentRecord) {
  return {
    ...record,
    models: [...record.models],
  };
}

function cloneToolDomainRecord(record: ToolCatalogDomainRecord): ToolCatalogDomainRecord {
  return { ...record, sampleTools: [...record.sampleTools] };
}

function cloneToolRecord(record: PlatformToolRecord): PlatformToolRecord {
  return { ...record };
}

function buildPlatformStatus(record: SystemAgentSeed) {
  if (!record.available) return "Unavailable" as const;
  return record.platformEnabled ? "Enabled" as const : "Disabled" as const;
}

function buildPlatformRecord(record: SystemAgentSeed): PlatformSystemAgentRecord {
  const status = buildPlatformStatus(record);
  return {
    agentCode: record.agentCode,
    available: record.available,
    description: record.description,
    enabled: status === "Enabled",
    lastScannedAt: record.lastScannedAt,
    models: [...record.models],
    name: record.name,
    source: record.source,
    status,
  };
}

function buildWorkspaceSystemRecord(workspaceCode: string, record: SystemAgentSeed): WorkspaceSystemAgentRecord {
  const workspaceAgentOverrides = workspaceSystemAgentOverrides[workspaceCode] ?? [];
  const workspaceOverrideDisabled = workspaceAgentOverrides.includes(record.agentCode);
  const enabled = record.available && record.platformEnabled && !workspaceOverrideDisabled;
  const status = !record.available
    ? "Unavailable"
    : enabled
      ? "Enabled"
      : "Disabled";

  return {
    agentCode: record.agentCode,
    available: record.available,
    description: record.description,
    enabled,
    inheritedFrom: "Platform",
    lastScannedAt: record.lastScannedAt,
    models: [...record.models],
    name: record.name,
    platformEnabled: record.platformEnabled,
    source: record.source,
    status,
    workspaceOverrideDisabled,
  };
}

function cloneWorkspaceSystemRecord(workspaceCode: string, record: SystemAgentSeed): WorkspaceSystemAgentRecord {
  const workspaceRecord = buildWorkspaceSystemRecord(workspaceCode, record);
  return {
    ...workspaceRecord,
    models: [...workspaceRecord.models],
  };
}

function normalizeToolCatalogDomain(record: ToolDomainSeed): ToolCatalogDomainRecord {
  const normalizedTools = toolSeed.filter((tool) => tool.domainCode === record.domainCode);
  const validTools = normalizedTools
    .filter((tool) => tool.operationKind)
    .map((tool) => tool.toolCode);
  const hasMutationTools = normalizedTools.some((tool) => tool.operationKind === "command");

  return {
    domainCode: record.domainCode,
    description: record.description,
    hasMutationTools,
    name: record.name,
    sampleTools: (record.sampleTools.length ? record.sampleTools : validTools).filter((toolCode) => validTools.includes(toolCode)),
    toolCount: normalizedTools.length,
  };
}

function getToolDomains() {
  return toolDomainSeed.map(normalizeToolCatalogDomain);
}

function getTools(): PlatformToolRecord[] {
  return toolSeed.map((tool) => ({ ...tool }));
}

function getWorkspaceInternalAgentOverrides(workspaceCode: string) {
  if (!workspaceOverrides[workspaceCode]) {
    workspaceOverrides[workspaceCode] = [];
  }
  return workspaceOverrides[workspaceCode];
}

function getWorkspaceInternalAgentOverride(workspaceCode: string, agentCode: string) {
  return getWorkspaceInternalAgentOverrides(workspaceCode).find((item) => item.agentCode === agentCode) ?? null;
}

function upsertWorkspaceInternalAgentOverride(workspaceCode: string, next: WorkspaceInternalAgentSeed) {
  const overrides = getWorkspaceInternalAgentOverrides(workspaceCode)
    .filter((item) => item.agentCode !== next.agentCode);
  workspaceOverrides = {
    ...workspaceOverrides,
    [workspaceCode]: [...overrides, next],
  };
}

function normalizeWorkspaceInternalAgentSelectedTools(toolCodes: string[] = []) {
  const validToolCodeSet = new Set(toolSeed.map((tool) => tool.toolCode));
  return [...new Set(toolCodes)].filter((toolCode) => validToolCodeSet.has(toolCode));
}

function buildWorkspaceInternalAgentRecord(workspaceCode: string, agentCode: string): WorkspaceInternalAgentRecord {
  const base = workspaceInternalAgentSeed.find((item) => item.agentCode === agentCode);
  if (!base) {
    throw new Error(`Internal agent ${agentCode} was not found.`);
  }

  const override = getWorkspaceInternalAgentOverride(workspaceCode, agentCode);
  const effective = override ? { ...base, ...override } : { ...base };
  const source = override ? "workspace_override" : "platform_default";
  const effectiveEnabledServerCodeSet = new Set(listEffectiveEnabledWorkspaceMcpServerCodesMock(workspaceCode));

  const selectedToolCodes = effective.toolAssignmentMode === "all_platform_tools"
    ? []
    : normalizeWorkspaceInternalAgentSelectedTools(effective.selectedToolCodes);
  const selectedServerCodes = effective.mcpAccessMode === "selected_servers"
    ? [...new Set(effective.selectedServerCodes)].filter((serverCode) => effectiveEnabledServerCodeSet.has(serverCode))
    : [];

  return {
    agentCode: effective.agentCode,
    description: effective.description,
    expandedDomainPersistenceMode: effective.expandedDomainPersistenceMode,
    maxSteps: effective.maxSteps,
    maxToolCalls: effective.maxToolCalls,
    maxParallelToolCalls: effective.maxParallelToolCalls,
    mcpAccessMode: effective.mcpAccessMode,
    name: effective.name,
    selectedServerCodes,
    selectedToolCodes,
    source,
    toolAssignmentMode: effective.toolAssignmentMode,
    toolExposureMode: effective.toolExposureMode,
  };
}

function listWorkspaceInternalAgentRecords(workspaceCode: string) {
  return workspaceInternalAgentSeed
    .map((agent) => buildWorkspaceInternalAgentRecord(workspaceCode, agent.agentCode));
}

function validateInternalAgentPatch(payload: WorkspaceInternalAgentPolicyPayload) {
  const invalidKey = Object.keys(payload).find((key) => {
    if ([
      "toolAssignmentMode",
      "toolExposureMode",
      "expandedDomainPersistenceMode",
      "maxSteps",
      "maxToolCalls",
      "maxParallelToolCalls",
      "mcpAccessMode",
    ].includes(key)) {
      return false;
    }
    return true;
  });

  if (invalidKey) {
    throw new Error(`Unsupported internal-agent policy field: ${invalidKey}`);
  }
}

export function getMainAgentConfigMock() {
  return cloneMainAgentConfig(mainAgentConfig);
}

export function updateMainAgentConfigMock(payload: MainAgentConfigPayload) {
  mainAgentConfig = {
    instructions: payload.instructions,
    personality: payload.personality,
    updatedAt: nextScanTimestamp(),
  };
  return cloneMainAgentConfig(mainAgentConfig);
}

export function listWorkspaceInternalAgentsMock(workspaceCode: string) {
  return listWorkspaceInternalAgentRecords(workspaceCode);
}

export function fetchWorkspaceInternalAgentMock(workspaceCode: string, agentCode: string) {
  return buildWorkspaceInternalAgentRecord(workspaceCode, agentCode);
}

export function patchWorkspaceInternalAgentPolicyMock(
  workspaceCode: string,
  agentCode: string,
  payload: WorkspaceInternalAgentPolicyPayload,
) {
  validateInternalAgentPatch(payload);
  const base = workspaceInternalAgentSeed.find((item) => item.agentCode === agentCode);
  if (!base) {
    throw new Error(`Internal agent ${agentCode} was not found.`);
  }

  const next: WorkspaceInternalAgentSeed = {
    ...base,
    ...getWorkspaceInternalAgentOverride(workspaceCode, agentCode),
    ...payload,
    agentCode,
    description: base.description,
    name: base.name,
  };

  upsertWorkspaceInternalAgentOverride(workspaceCode, next);
  return buildWorkspaceInternalAgentRecord(workspaceCode, agentCode);
}

export function patchWorkspaceInternalAgentMcpPolicyMock(
  workspaceCode: string,
  agentCode: string,
  payload: WorkspaceInternalAgentMcpPolicyPayload,
) {
  const base = workspaceInternalAgentSeed.find((item) => item.agentCode === agentCode);
  if (!base) {
    throw new Error(`Internal agent ${agentCode} was not found.`);
  }

  const next: WorkspaceInternalAgentSeed = {
    ...base,
    ...getWorkspaceInternalAgentOverride(workspaceCode, agentCode),
    ...payload,
    agentCode,
    description: base.description,
    name: base.name,
  };

  upsertWorkspaceInternalAgentOverride(workspaceCode, next);
  return buildWorkspaceInternalAgentRecord(workspaceCode, agentCode);
}

export function updateWorkspaceInternalAgentToolAssignmentsMock(
  workspaceCode: string,
  agentCode: string,
  toolCodes: string[],
) {
  const base = workspaceInternalAgentSeed.find((item) => item.agentCode === agentCode);
  if (!base) {
    throw new Error(`Internal agent ${agentCode} was not found.`);
  }

  const next: WorkspaceInternalAgentSeed = {
    ...base,
    ...getWorkspaceInternalAgentOverride(workspaceCode, agentCode),
    agentCode,
    selectedToolCodes: normalizeWorkspaceInternalAgentSelectedTools(toolCodes),
    toolAssignmentMode: "selected_tools",
  };

  upsertWorkspaceInternalAgentOverride(workspaceCode, next);
  return buildWorkspaceInternalAgentRecord(workspaceCode, agentCode);
}

export function updateWorkspaceInternalAgentMcpServerAssignmentsMock(
  workspaceCode: string,
  agentCode: string,
  serverCodes: string[],
) {
  const base = workspaceInternalAgentSeed.find((item) => item.agentCode === agentCode);
  if (!base) {
    throw new Error(`Internal agent ${agentCode} was not found.`);
  }

  const allowedServerCodes = new Set(listEffectiveEnabledWorkspaceMcpServerCodesMock(workspaceCode));
  const normalizedServerCodes = [...new Set(serverCodes)].filter((serverCode) => allowedServerCodes.has(serverCode));
  if (normalizedServerCodes.length !== [...new Set(serverCodes)].length) {
    throw new Error("One or more selected MCP servers are no longer effective in this workspace.");
  }

  const next: WorkspaceInternalAgentSeed = {
    ...base,
    ...getWorkspaceInternalAgentOverride(workspaceCode, agentCode),
    agentCode,
    mcpAccessMode: "selected_servers",
    selectedServerCodes: normalizedServerCodes,
  };

  upsertWorkspaceInternalAgentOverride(workspaceCode, next);
  return buildWorkspaceInternalAgentRecord(workspaceCode, agentCode);
}

export function resetWorkspaceInternalAgentPolicyMock(workspaceCode: string, agentCode: string) {
  const overrides = getWorkspaceInternalAgentOverrides(workspaceCode)
    .filter((override) => override.agentCode !== agentCode);
  workspaceOverrides = {
    ...workspaceOverrides,
    [workspaceCode]: [...overrides],
  };
  return buildWorkspaceInternalAgentRecord(workspaceCode, agentCode);
}

export function listToolDomainsMock(query: ListToolsQuery = {}) {
  const normalizedSearch = query.search?.trim().toLowerCase() ?? "";
  const domains = getToolDomains();
  if (!normalizedSearch) {
    return domains.map(cloneToolDomainRecord);
  }

  return domains
    .filter((domain) => [
      domain.domainCode,
      domain.name,
      domain.description,
    ].join(" ").toLowerCase().includes(normalizedSearch))
    .map(cloneToolDomainRecord);
}

export function listToolsMock(payload: ListToolsQuery = {}) {
  const normalizedSearch = payload.search?.trim().toLowerCase() ?? "";
  const domainFilter = payload.domainCode?.trim();
  const normalizedDomainFilter = domainFilter?.toLowerCase() ?? "";

  return getTools()
    .filter((tool) => {
      if (domainFilter && tool.domainCode.toLowerCase() !== normalizedDomainFilter) {
        return false;
      }

      if (!normalizedSearch) {
        return true;
      }

      return [
        tool.toolCode,
        tool.domainCode,
        tool.name,
        tool.description,
        tool.operationKind,
      ].join(" ").toLowerCase().includes(normalizedSearch);
    })
    .map(cloneToolRecord);
}

export function listWorkspaceToolDomainsMock(query: ListToolsQuery = {}) {
  return listToolDomainsMock(query);
}

export function listWorkspaceToolsMock(payload: ListToolsQuery = {}) {
  return listToolsMock(payload);
}

export function listPlatformSystemAgentsMock() {
  return systemAgentRegistry.map((record) => cloneSystemAgentRecord(buildPlatformRecord(record)));
}

export function rescanPlatformSystemAgentsMock() {
  const scannedAt = nextScanTimestamp();
  systemAgentRegistry.forEach((record) => {
    record.lastScannedAt = scannedAt;
    if (record.agentCode === "opencode") {
      record.available = false;
    }
  });

  const records = listWorkspaceSystemAgentsMock("TRON_CORE");
  return {
    availableCount: records.filter((record) => record.available).length,
    records,
    scannedCount: records.length,
    unavailableCount: records.filter((record) => !record.available).length,
  } satisfies PlatformSystemAgentRescanResult;
}

export function setPlatformSystemAgentEnabledMock(agentCode: string, enabled: boolean) {
  const record = systemAgentRegistry.find((item) => item.agentCode === agentCode);
  if (!record) {
    throw new Error(`System agent ${agentCode} was not found.`);
  }
  if (!record.available) {
    throw new Error(`${record.name} is unavailable and cannot be updated.`);
  }

  record.platformEnabled = enabled;
  return cloneSystemAgentRecord(buildPlatformRecord(record));
}

export function listWorkspaceSystemAgentsMock(workspaceCode: string) {
  if (!workspaceSystemAgentOverrides[workspaceCode]) {
    workspaceSystemAgentOverrides[workspaceCode] = [];
  }
  return systemAgentRegistry
    .map((record) => cloneWorkspaceSystemRecord(workspaceCode, record));
}

export function setWorkspaceSystemAgentEnabledMock(
  workspaceCode: string,
  agentCode: string,
  enabled: boolean,
) {
  const record = systemAgentRegistry.find((item) => item.agentCode === agentCode);
  if (!record) {
    throw new Error(`System agent ${agentCode} was not found.`);
  }
  if (!record.available) {
    throw new Error(`${record.name} is unavailable and cannot be updated.`);
  }

  const overrides = new Set(workspaceSystemAgentOverrides[workspaceCode] ?? []);
  if (enabled) {
    overrides.delete(agentCode);
  } else {
    overrides.add(agentCode);
  }

  workspaceSystemAgentOverrides = {
    ...workspaceSystemAgentOverrides,
    [workspaceCode]: [...overrides],
  };

  return cloneWorkspaceSystemRecord(workspaceCode, record);
}
