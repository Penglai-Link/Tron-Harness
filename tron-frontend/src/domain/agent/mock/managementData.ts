import type { MainAgentConfigPayload } from "@/domain/agent/api/agentManagementApi";
import type {
  MainAgentConfig,
  PlatformSystemAgentRecord,
  PlatformSystemAgentRescanResult,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";

type SystemAgentSeed = {
  agentCode: string;
  available: boolean;
  description: string;
  name: string;
  platformEnabled: boolean;
  source: string;
  lastScannedAt: string | null;
};

const scanBaseMs = Date.parse("2026-05-29T09:30:00Z");
let scanSequence = 0;

let mainAgentConfig: MainAgentConfig = {
  personality: "Direct, accountable, and precise. Push for concrete next steps and expose tradeoffs plainly.",
  instructions: "Coordinate coding tasks with clear ownership boundaries, keep outputs concise, and require validation before close-out.",
  updatedAt: "2026-05-29T09:30:00Z",
};

let systemAgentRegistry: SystemAgentSeed[] = [
  {
    agentCode: "codex",
    available: true,
    description: "OpenAI coding agent runtime used for structured feature delivery.",
    name: "Codex",
    platformEnabled: true,
    source: "System Scan",
    lastScannedAt: "2026-05-29T09:30:00Z",
  },
  {
    agentCode: "opencode",
    available: true,
    description: "Local CLI coding agent kept as a platform-level fallback executor.",
    name: "OpenCode",
    platformEnabled: false,
    source: "System Scan",
    lastScannedAt: "2026-05-29T09:30:00Z",
  },
];

let workspaceDisabledOverrides: Record<string, string[]> = {
  COMPLIANCE: [],
  ENTERPRISE: [],
  PLATFORM: [],
  TRON_CORE: [],
};

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

function clonePlatformAgentRecord(record: PlatformSystemAgentRecord): PlatformSystemAgentRecord {
  return { ...record };
}

function normalizeWorkspaceOverrides(workspaceCode: string) {
  if (!workspaceDisabledOverrides[workspaceCode]) {
    workspaceDisabledOverrides = {
      ...workspaceDisabledOverrides,
      [workspaceCode]: [],
    };
  }

  return workspaceDisabledOverrides[workspaceCode];
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
    name: record.name,
    source: record.source,
    status,
  };
}

function buildWorkspaceRecord(
  workspaceCode: string,
  record: SystemAgentSeed,
): WorkspaceSystemAgentRecord {
  const overrides = normalizeWorkspaceOverrides(workspaceCode);
  const workspaceOverrideDisabled = overrides.includes(record.agentCode);
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
    name: record.name,
    platformEnabled: record.platformEnabled,
    source: record.source,
    status,
    workspaceOverrideDisabled,
  };
}

function findSystemAgentSeed(agentCode: string) {
  const record = systemAgentRegistry.find((item) => item.agentCode === agentCode);
  if (!record) {
    throw new Error(`System agent ${agentCode} was not found.`);
  }

  return record;
}

function assertSystemAgentMutable(record: SystemAgentSeed) {
  if (!record.available) {
    throw new Error(`${record.name} is unavailable and cannot be updated.`);
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

export function listPlatformSystemAgentsMock() {
  return systemAgentRegistry.map((record) => clonePlatformAgentRecord(buildPlatformRecord(record)));
}

export function rescanPlatformSystemAgentsMock() {
  const scannedAt = nextScanTimestamp();
  systemAgentRegistry = systemAgentRegistry.map((record) => ({
    ...record,
    available: record.agentCode === "opencode" ? false : record.available,
    lastScannedAt: scannedAt,
  }));

  const records = listPlatformSystemAgentsMock();
  return {
    availableCount: records.filter((record) => record.available).length,
    records,
    scannedCount: records.length,
    unavailableCount: records.filter((record) => !record.available).length,
  } satisfies PlatformSystemAgentRescanResult;
}

export function setPlatformSystemAgentEnabledMock(agentCode: string, enabled: boolean) {
  const record = findSystemAgentSeed(agentCode);
  assertSystemAgentMutable(record);
  systemAgentRegistry = systemAgentRegistry.map((item) => (
    item.agentCode === agentCode
      ? { ...item, platformEnabled: enabled }
      : item
  ));
  return buildPlatformRecord(findSystemAgentSeed(agentCode));
}

export function listWorkspaceSystemAgentsMock(workspaceCode: string) {
  return systemAgentRegistry.map((record) => buildWorkspaceRecord(workspaceCode, record));
}

export function setWorkspaceSystemAgentEnabledMock(
  workspaceCode: string,
  agentCode: string,
  enabled: boolean,
) {
  const record = findSystemAgentSeed(agentCode);
  assertSystemAgentMutable(record);
  const overrides = normalizeWorkspaceOverrides(workspaceCode);
  const overrideSet = new Set(overrides);

  if (enabled) {
    overrideSet.delete(agentCode);
  } else {
    overrideSet.add(agentCode);
  }

  workspaceDisabledOverrides = {
    ...workspaceDisabledOverrides,
    [workspaceCode]: [...overrideSet],
  };

  return buildWorkspaceRecord(workspaceCode, record);
}
