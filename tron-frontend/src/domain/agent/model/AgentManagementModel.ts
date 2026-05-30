export type AgentStatus = "Enabled" | "Disabled" | "Unavailable";

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
