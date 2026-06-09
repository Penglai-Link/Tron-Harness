export type AgentStatus = "Enabled" | "Disabled" | "Unavailable";

export type ToolAssignmentMode = "all_platform_tools" | "selected_tools";
export type ToolExposureMode = "direct_list" | "domain_discovery";
export type ExpandedDomainPersistenceMode = "sticky_expanded" | "ephemeral_expanded";
export type WorkspaceInternalAgentPolicySource = "platform_default" | "workspace_override";
export type McpAccessMode = "disabled" | "all_enabled_servers" | "selected_servers";

export interface SystemAgentModelRecord {
  modelId: string;
  displayName: string;
  contextWindowTokens?: number | null;
}

export interface MainAgentConfig {
  personality: string;
  instructions: string;
  updatedAt: string;
}

interface SystemAgentBaseRecord {
  agentCode: string;
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
  toolCode: string;
  domainCode: string;
  name: string;
  description: string;
  operationKind: "query" | "command";
}

export interface WorkspaceInternalAgentRecord {
  agentCode: string;
  name: string;
  description: string;
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
