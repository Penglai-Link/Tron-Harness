import type { ReasoningEffort } from "@/domain/model/model/ModelRegistryModel";
export type AgentStatus = "Enabled" | "Disabled" | "Unavailable";

export type ToolAssignmentMode = "all_platform_tools" | "selected_tools" | "all_read_only_tools";
export type ToolExposureMode = "direct_list" | "domain_discovery";
export type ExpandedDomainPersistenceMode =
  | "sticky_expanded"
  | "ephemeral_expanded";
export type WorkspaceInternalAgentPolicySource =
  | "platform_default"
  | "workspace_override";
export type McpAccessMode =
  | "disabled"
  | "all_enabled_servers"
  | "selected_servers";
// Client configuration scope; never sent as a workspace identifier to the backend.
export const PLATFORM_INTERNAL_AGENT_SCOPE = "$platform";

export interface SystemAgentModelRecord {
  supportedReasoningEfforts?: ReasoningEffort[];
  modelId: string;
  displayName: string;
  contextWindowTokens?: number | null;
}

interface SystemAgentBaseRecord {
  agentCode: string;
  baseAgentCode?: string | null;
  modelId?: string | null;
  fastMode?: boolean;
  reasoningEffort?: ReasoningEffort | null;
  avatarId?: string | null;
  name: string;
  description: string;
  source: string;
  status: AgentStatus;
  enabled: boolean;
  available: boolean;
  models: SystemAgentModelRecord[];
  lastScannedAt: string | null;
}

export interface PlatformSystemAgentRecord extends SystemAgentBaseRecord {}

export interface SystemAgentProfilePayload {
  agentCode: string;
  name: string;
  description?: string;
  baseAgentCode: string;
  modelId: string;
  fastMode?: boolean;
  reasoningEffort?: ReasoningEffort | null;
  avatarId?: string | null;
}

export interface PlatformSystemAgentRescanResult {
  scannedCount: number;
  availableCount: number;
  unavailableCount: number;
  records: PlatformSystemAgentRecord[];
}

export interface WorkspaceSystemAgentRecord extends SystemAgentBaseRecord {
  inheritedFrom: "Platform";
  platformEnabled: boolean;
  workspaceOverrideDisabled: boolean;
}

export interface ToolCatalogDomainRecord {
  domainCode: string;
  name: string;
  description: string;
  toolCount: number;
  sampleTools: string[];
  hasMutationTools: boolean;
}

export interface PlatformToolRecord {
  providerPackId?: string | null;
  assignable?: boolean;
  disabledReason?: string | null;
  toolCode: string;
  toolName: string;
  domainCode: string;
  name: string;
  description: string;
  operationKind: "query" | "command" | "unknown";
}

export interface WorkspaceInternalAgentRecord {
  enabled?: boolean;
  agentCode: string;
  name: string;
  description: string;
  rolePrompt: string;
  loopCode?: string | null;
  avatarId?: string;
  candidateModelCodes?: string[];
  groupName?: string | null;
  platformDefined: boolean;
  builtIn?: boolean;
  mcpAccessMode: McpAccessMode;
  toolAssignmentMode: ToolAssignmentMode;
  toolExposureMode: ToolExposureMode;
  expandedDomainPersistenceMode: ExpandedDomainPersistenceMode;
  maxSteps: number;
  maxToolCalls: number;
  maxParallelToolCalls: number;
  selectedServerCodes: string[];
  selectedToolCodes: string[];
  source: WorkspaceInternalAgentPolicySource;
}

export interface CreateInternalAgentPayload {
  agentCode: string;
  name: string;
  description?: string;
  rolePrompt: string;
  loopCode?: string | null;
  avatarId?: string;
  candidateModelCodes?: string[];
  groupName?: string | null;
}

export interface UpdateInternalAgentDefinitionPayload {
  enabled?: boolean;
  name?: string;
  description?: string;
  rolePrompt?: string;
  loopCode?: string | null;
  avatarId?: string;
  candidateModelCodes?: string[];
  groupName?: string | null;
}

export interface WorkspaceInternalAgentPolicyPayload {
  toolAssignmentMode?: ToolAssignmentMode;
  toolExposureMode?: ToolExposureMode;
  expandedDomainPersistenceMode?: ExpandedDomainPersistenceMode;
  maxSteps?: number;
  maxToolCalls?: number;
  maxParallelToolCalls?: number;
}

export interface WorkspaceInternalAgentMcpPolicyPayload {
  mcpAccessMode?: McpAccessMode;
}

export interface ListToolsQuery {
  search?: string;
  domainCode?: string;
}
