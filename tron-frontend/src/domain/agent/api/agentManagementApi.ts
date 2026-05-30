import { requestApi } from "@/shared/api/httpClient";

import type {
  MainAgentConfig,
  PlatformSystemAgentRecord,
  PlatformSystemAgentRescanResult,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";

import { agentApiEndpoints } from "./agentApi";

export interface MainAgentConfigPayload {
  personality: string;
  instructions: string;
}

type AgentStatusApi = "ENABLED" | "DISABLED" | "UNAVAILABLE" | PlatformSystemAgentRecord["status"];
type AgentSourceApi = "SYSTEM" | PlatformSystemAgentRecord["source"];

interface ManagedSystemAgentRecordApi {
  agentCode: string;
  name: string;
  description: string;
  source: AgentSourceApi;
  status: AgentStatusApi;
  enabled: boolean;
  available: boolean;
  lastScannedAt: string | null;
  inheritedFrom?: string | null;
  platformEnabled?: boolean;
  workspaceOverrideDisabled?: boolean;
}

interface PlatformSystemAgentRescanResultApi {
  scannedCount: number;
  availableCount: number;
  unavailableCount: number;
  records: ManagedSystemAgentRecordApi[];
}

function normalizeAgentStatus(status: AgentStatusApi): PlatformSystemAgentRecord["status"] {
  switch (status) {
    case "ENABLED":
      return "Enabled";
    case "DISABLED":
      return "Disabled";
    case "UNAVAILABLE":
      return "Unavailable";
    default:
      return status;
  }
}

function normalizeAgentSource(source: AgentSourceApi) {
  if (source === "SYSTEM") {
    return "System Scan";
  }

  return source;
}

function normalizePlatformSystemAgentRecord(
  record: ManagedSystemAgentRecordApi,
): PlatformSystemAgentRecord {
  return {
    agentCode: record.agentCode,
    available: record.available,
    description: record.description,
    enabled: record.enabled,
    lastScannedAt: record.lastScannedAt,
    name: record.name,
    source: normalizeAgentSource(record.source),
    status: normalizeAgentStatus(record.status),
  };
}

function normalizeWorkspaceSystemAgentRecord(
  record: ManagedSystemAgentRecordApi,
): WorkspaceSystemAgentRecord {
  return {
    agentCode: record.agentCode,
    available: record.available,
    description: record.description,
    enabled: record.enabled,
    inheritedFrom: (record.inheritedFrom ?? "Platform") as WorkspaceSystemAgentRecord["inheritedFrom"],
    lastScannedAt: record.lastScannedAt,
    name: record.name,
    platformEnabled: record.platformEnabled ?? record.enabled,
    source: normalizeAgentSource(record.source),
    status: normalizeAgentStatus(record.status),
    workspaceOverrideDisabled: record.workspaceOverrideDisabled ?? false,
  };
}

export function fetchMainAgentConfig(signal?: AbortSignal) {
  return requestApi<MainAgentConfig>(agentApiEndpoints.getMainAgentConfig, { signal });
}

export function updateMainAgentConfig(payload: MainAgentConfigPayload) {
  return requestApi<MainAgentConfig>(agentApiEndpoints.updateMainAgentConfig, {
    body: payload,
  });
}

export function fetchPlatformSystemAgents(signal?: AbortSignal) {
  return requestApi<ManagedSystemAgentRecordApi[]>(
    agentApiEndpoints.listPlatformSystemAgents,
    { signal },
  ).then((records) => records.map(normalizePlatformSystemAgentRecord));
}

export function rescanPlatformSystemAgents() {
  return requestApi<PlatformSystemAgentRescanResultApi>(agentApiEndpoints.rescanPlatformSystemAgents, {
    body: {},
  }).then((result): PlatformSystemAgentRescanResult => ({
    ...result,
    records: result.records.map(normalizePlatformSystemAgentRecord),
  }));
}

export function enablePlatformSystemAgent(agentCode: string) {
  return requestApi<ManagedSystemAgentRecordApi>(agentApiEndpoints.enablePlatformSystemAgent, {
    pathParams: { agentCode },
  }).then(normalizePlatformSystemAgentRecord);
}

export function disablePlatformSystemAgent(agentCode: string) {
  return requestApi<ManagedSystemAgentRecordApi>(agentApiEndpoints.disablePlatformSystemAgent, {
    pathParams: { agentCode },
  }).then(normalizePlatformSystemAgentRecord);
}

export function fetchWorkspaceSystemAgents(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<ManagedSystemAgentRecordApi[]>(
    agentApiEndpoints.listWorkspaceSystemAgents,
    {
      pathParams: { workspaceCode },
      signal,
    },
  ).then((records) => records.map(normalizeWorkspaceSystemAgentRecord));
}

export function enableWorkspaceSystemAgent(workspaceCode: string, agentCode: string) {
  return requestApi<ManagedSystemAgentRecordApi>(agentApiEndpoints.enableWorkspaceSystemAgent, {
    pathParams: { workspaceCode, agentCode },
  }).then(normalizeWorkspaceSystemAgentRecord);
}

export function disableWorkspaceSystemAgent(workspaceCode: string, agentCode: string) {
  return requestApi<ManagedSystemAgentRecordApi>(agentApiEndpoints.disableWorkspaceSystemAgent, {
    pathParams: { workspaceCode, agentCode },
  }).then(normalizeWorkspaceSystemAgentRecord);
}
