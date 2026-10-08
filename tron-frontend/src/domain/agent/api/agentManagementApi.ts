import { requestApi } from "@/shared/api/httpClient";

import type {
  PlatformToolRecord,
  ToolCatalogDomainRecord,
  SystemAgentModelRecord,
  PlatformSystemAgentRecord,
  PlatformSystemAgentRescanResult,
  WorkspaceSystemAgentRecord,
  WorkspaceInternalAgentMcpPolicyPayload,
  WorkspaceInternalAgentPolicyPayload,
  WorkspaceInternalAgentRecord,
  ListToolsQuery,
  CreateInternalAgentPayload,
  UpdateInternalAgentDefinitionPayload,
} from "@/domain/agent/model";

import { agentApiEndpoints } from "./agentApi";
import { PLATFORM_INTERNAL_AGENT_SCOPE } from "../model";

function internalAgentEndpoint(endpoint: string, workspaceCode: string) {
  return workspaceCode === PLATFORM_INTERNAL_AGENT_SCOPE
    ? endpoint.replace("/workspaces/{workspaceCode}", "/registry")
    : endpoint;
}


export interface ToolListPayload {
  domainCode?: string;
  search?: string;
}

type AgentStatusApi = "ENABLED" | "DISABLED" | "UNAVAILABLE" | PlatformSystemAgentRecord["status"];
type AgentSourceApi = "SYSTEM" | PlatformSystemAgentRecord["source"];
type ToolOperationKindApi = PlatformToolRecord["operationKind"] | "QUERY" | "COMMAND";
type ToolAssignmentModeApi = WorkspaceInternalAgentRecord["toolAssignmentMode"] | Uppercase<WorkspaceInternalAgentRecord["toolAssignmentMode"]>;
type ToolExposureModeApi = WorkspaceInternalAgentRecord["toolExposureMode"] | Uppercase<WorkspaceInternalAgentRecord["toolExposureMode"]>;
type ExpandedDomainPersistenceModeApi = WorkspaceInternalAgentRecord["expandedDomainPersistenceMode"] | Uppercase<WorkspaceInternalAgentRecord["expandedDomainPersistenceMode"]>;
type WorkspaceInternalAgentSourceApi = WorkspaceInternalAgentRecord["source"] | "PLATFORM_DEFAULT" | "WORKSPACE_OVERRIDE";
type McpAccessModeApi = WorkspaceInternalAgentRecord["mcpAccessMode"] | Uppercase<WorkspaceInternalAgentRecord["mcpAccessMode"]>;

interface ManagedSystemAgentRecordApi {
  agentCode: string;
  name: string;
  description: string;
  source: AgentSourceApi;
  status: AgentStatusApi;
  enabled: boolean;
  available: boolean;
  models?: SystemAgentModelRecord[] | null;
  lastScannedAt: string | null;
  inheritedFrom?: string | null;
  platformEnabled?: boolean;
  workspaceOverrideDisabled?: boolean;
}

interface ToolCatalogDomainRecordApi {
  domainCode: string;
  name: string;
  description: string;
  toolCount: number;
  sampleTools?: string[] | null;
  hasMutationTools?: boolean;
}

interface ToolRecordApi {
  toolCode: string;
  toolName: string;
  domainCode: string;
  name: string;
  description: string;
  operationKind: ToolOperationKindApi;
}

interface WorkspaceInternalAgentRecordApi {
  agentCode: string;
  name: string;
  description: string;
  rolePrompt: string;
  platformDefined: boolean;
  source?: WorkspaceInternalAgentSourceApi;
  mcpAccessMode?: McpAccessModeApi;
  toolAssignmentMode: ToolAssignmentModeApi;
  toolExposureMode: ToolExposureModeApi;
  expandedDomainPersistenceMode: ExpandedDomainPersistenceModeApi;
  maxSteps: number;
  maxToolCalls: number;
  maxParallelToolCalls?: number;
  selectedServerCodes?: string[];
  selectedToolCodes: string[];
}

function normalizeAgentModels(models: ManagedSystemAgentRecordApi["models"]) {
  if (!Array.isArray(models)) return [];
  return models
    .map((model) => ({
      displayName: model.displayName,
      modelId: model.modelId,
    }))
    .filter((model) => model.modelId?.trim().length > 0)
    .map((model) => ({
      displayName: model.displayName?.trim() || model.modelId.trim(),
      modelId: model.modelId.trim(),
    }));
}

interface PlatformSystemAgentRescanResultApi {
  scannedCount: number;
  availableCount: number;
  unavailableCount: number;
  records: ManagedSystemAgentRecordApi[];
}

function normalizeToolOperationKind(kind: ToolOperationKindApi): PlatformToolRecord["operationKind"] {
  switch (kind) {
    case "COMMAND":
      return "command";
    case "command":
      return kind;
    case "QUERY":
      return "query";
    case "query":
      return kind;
    default:
      return "query";
  }
}

function normalizeToolAssignmentMode(mode: ToolAssignmentModeApi): WorkspaceInternalAgentRecord["toolAssignmentMode"] {
  switch (mode) {
    case "ALL_PLATFORM_TOOLS":
    case "all_platform_tools":
      return "all_platform_tools";
    case "SELECTED_TOOLS":
    case "selected_tools":
      return "selected_tools";
    default:
      return "all_platform_tools";
  }
}

function normalizeToolExposureMode(mode: ToolExposureModeApi): WorkspaceInternalAgentRecord["toolExposureMode"] {
  switch (mode) {
    case "DOMAIN_DISCOVERY":
    case "domain_discovery":
      return "domain_discovery";
    case "DIRECT_LIST":
    case "direct_list":
      return "direct_list";
    default:
      return "direct_list";
  }
}

function normalizeExpandedDomainPersistenceMode(
  mode: ExpandedDomainPersistenceModeApi,
): WorkspaceInternalAgentRecord["expandedDomainPersistenceMode"] {
  switch (mode) {
    case "EPHEMERAL_EXPANDED":
    case "ephemeral_expanded":
      return "ephemeral_expanded";
    case "STICKY_EXPANDED":
    case "sticky_expanded":
      return "sticky_expanded";
    default:
      return "sticky_expanded";
  }
}

function normalizeWorkspaceInternalAgentSource(
  source: WorkspaceInternalAgentSourceApi | undefined,
): WorkspaceInternalAgentRecord["source"] {
  switch (source) {
    case "WORKSPACE_OVERRIDE":
      return "workspace_override";
    case "PLATFORM_DEFAULT":
      return "platform_default";
    default:
      return "platform_default";
  }
}

function normalizeMcpAccessMode(mode: McpAccessModeApi | undefined): WorkspaceInternalAgentRecord["mcpAccessMode"] {
  switch (mode) {
    case "ALL_ENABLED_SERVERS":
    case "all_enabled_servers":
      return "all_enabled_servers";
    case "SELECTED_SERVERS":
    case "selected_servers":
      return "selected_servers";
    default:
      return "disabled";
  }
}

function normalizeToolCatalogDomainRecord(record: ToolCatalogDomainRecordApi): ToolCatalogDomainRecord {
  return {
    description: record.description,
    domainCode: record.domainCode,
    hasMutationTools: Boolean(record.hasMutationTools),
    name: record.name,
    sampleTools: Array.isArray(record.sampleTools)
      ? record.sampleTools.filter((toolCode) => toolCode.trim().length > 0)
      : [],
    toolCount: Number.isFinite(record.toolCount) ? record.toolCount : 0,
  };
}

function normalizeToolRecord(record: ToolRecordApi): PlatformToolRecord {
  return {
    description: record.description,
    domainCode: record.domainCode,
    name: record.name,
    operationKind: normalizeToolOperationKind(record.operationKind),
    toolCode: record.toolCode,
    toolName: record.toolName,
  };
}

function normalizeWorkspaceInternalAgentRecord(record: WorkspaceInternalAgentRecordApi): WorkspaceInternalAgentRecord {
  return {
    agentCode: record.agentCode,
    description: record.description,
    rolePrompt: record.rolePrompt ?? "",
    platformDefined: record.platformDefined ?? record.agentCode === "main-agent",
    expandedDomainPersistenceMode: normalizeExpandedDomainPersistenceMode(record.expandedDomainPersistenceMode),
    mcpAccessMode: normalizeMcpAccessMode(record.mcpAccessMode),
    name: record.name,
    maxSteps: typeof record.maxSteps === "number" && Number.isFinite(record.maxSteps) ? record.maxSteps : 100,
    maxToolCalls: typeof record.maxToolCalls === "number" && Number.isFinite(record.maxToolCalls) ? record.maxToolCalls : 100,
    maxParallelToolCalls:
      typeof record.maxParallelToolCalls === "number" && Number.isFinite(record.maxParallelToolCalls)
        ? record.maxParallelToolCalls
        : 4,
    source: normalizeWorkspaceInternalAgentSource(record.source),
    selectedServerCodes: Array.isArray(record.selectedServerCodes) ? [...new Set(record.selectedServerCodes)] : [],
    selectedToolCodes: Array.isArray(record.selectedToolCodes) ? [...new Set(record.selectedToolCodes)] : [],
    toolAssignmentMode: normalizeToolAssignmentMode(record.toolAssignmentMode),
    toolExposureMode: normalizeToolExposureMode(record.toolExposureMode),
  };
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
    models: normalizeAgentModels(record.models),
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
    models: normalizeAgentModels(record.models),
    name: record.name,
    platformEnabled: record.platformEnabled ?? record.enabled,
    source: normalizeAgentSource(record.source),
    status: normalizeAgentStatus(record.status),
    workspaceOverrideDisabled: record.workspaceOverrideDisabled ?? false,
  };
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

export function fetchToolDomains(signal?: AbortSignal) {
  return requestApi<ToolCatalogDomainRecordApi[]>(agentApiEndpoints.getToolDomains, { signal }).then(
    (records) => records.map(normalizeToolCatalogDomainRecord),
  );
}

export function listTools(payload: ListToolsQuery = {}, signal?: AbortSignal) {
  return requestApi<ToolRecordApi[]>(agentApiEndpoints.getTools, {
    queryParams: {
      domainCode: payload.domainCode,
      search: payload.search,
    },
    signal,
  }).then((records) => records.map(normalizeToolRecord));
}

export function fetchWorkspaceInternalAgents(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<WorkspaceInternalAgentRecordApi[]>(internalAgentEndpoint(agentApiEndpoints.listWorkspaceInternalAgents, workspaceCode), {
    pathParams: { workspaceCode },
    signal,
  }).then((records) => records.map(normalizeWorkspaceInternalAgentRecord));
}

export function createWorkspaceInternalAgent(workspaceCode: string, payload: CreateInternalAgentPayload) {
  return requestApi<{ agentCode: string; name: string; rolePrompt: string }>(
    internalAgentEndpoint("POST /api/v1/agents/workspaces/{workspaceCode}/internal", workspaceCode),
    { pathParams: { workspaceCode }, body: payload },
  );
}

export function patchWorkspaceInternalAgentDefinition(workspaceCode: string, agentCode: string, payload: UpdateInternalAgentDefinitionPayload) {
  return requestApi<{ agentCode: string; name: string; rolePrompt: string }>(
    internalAgentEndpoint("PATCH /api/v1/agents/workspaces/{workspaceCode}/internal/{agentCode}/definition", workspaceCode),
    { pathParams: { workspaceCode, agentCode }, body: payload },
  );
}

export function fetchWorkspaceInternalAgent(
  workspaceCode: string,
  agentCode: string,
  signal?: AbortSignal,
) {
  return requestApi<WorkspaceInternalAgentRecordApi>(internalAgentEndpoint(agentApiEndpoints.getWorkspaceInternalAgent, workspaceCode), {
    pathParams: { workspaceCode, agentCode },
    signal,
  }).then(normalizeWorkspaceInternalAgentRecord);
}

export function patchWorkspaceInternalAgentPolicy(
  workspaceCode: string,
  agentCode: string,
  payload: WorkspaceInternalAgentPolicyPayload,
) {
  return requestApi<WorkspaceInternalAgentRecordApi>(internalAgentEndpoint(agentApiEndpoints.patchWorkspaceInternalAgentPolicy, workspaceCode), {
    pathParams: { workspaceCode, agentCode },
    body: payload,
  }).then(normalizeWorkspaceInternalAgentRecord);
}

export function updateWorkspaceInternalAgentToolAssignments(workspaceCode: string, agentCode: string, toolCodes: string[]) {
  return requestApi<WorkspaceInternalAgentRecordApi>(internalAgentEndpoint(agentApiEndpoints.updateWorkspaceInternalAgentToolAssignments, workspaceCode), {
    pathParams: { workspaceCode, agentCode },
    body: { toolCodes },
  }).then(normalizeWorkspaceInternalAgentRecord);
}

export function patchWorkspaceInternalAgentMcpPolicy(
  workspaceCode: string,
  agentCode: string,
  payload: WorkspaceInternalAgentMcpPolicyPayload,
) {
  return requestApi<WorkspaceInternalAgentRecordApi>(internalAgentEndpoint(agentApiEndpoints.patchWorkspaceInternalAgentMcpPolicy, workspaceCode), {
    pathParams: { workspaceCode, agentCode },
    body: payload,
  }).then(normalizeWorkspaceInternalAgentRecord);
}

export function updateWorkspaceInternalAgentMcpServerAssignments(
  workspaceCode: string,
  agentCode: string,
  serverCodes: string[],
) {
  return requestApi<WorkspaceInternalAgentRecordApi>(internalAgentEndpoint(agentApiEndpoints.updateWorkspaceInternalAgentMcpServerAssignments, workspaceCode), {
    pathParams: { workspaceCode, agentCode },
    body: { serverCodes },
  }).then(normalizeWorkspaceInternalAgentRecord);
}

export function resetWorkspaceInternalAgentPolicy(workspaceCode: string, agentCode: string) {
  return requestApi<WorkspaceInternalAgentRecordApi>(agentApiEndpoints.resetWorkspaceInternalAgentPolicy, {
    pathParams: { workspaceCode, agentCode },
  }).then(normalizeWorkspaceInternalAgentRecord);
}
