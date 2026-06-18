export interface SkillSchemaPreview {
  name: string;
  fields: { name: string; type: string; required: boolean }[];
}

export type SkillSearchMode = "find" | "repo";

export interface CapabilityRecord {
  id: string;
  mode: "skills" | "mcp";
  name: string;
  description: string;
  provider: string;
  status: string;
  enabled: boolean;
  latency: string;
  schema: string;
  activity: string[];
}

export type SkillSourceType =
  | "Marketplace"
  | "GitHub"
  | "Git"
  | "Url"
  | "Local"
  | "Manual";

export type SkillUpdateStatus =
  | "Unknown"
  | "UpToDate"
  | "UpdateAvailable"
  | "Manual"
  | "Error";

export type SkillManagedBy = "Cli" | "Manual";

export type SkillWorkspaceSourceKind =
  | "Registry"
  | "Local"
  | "Inherited"
  | "Override";

export type SkillRegistryState = "Present" | "Deleted";

export type SkillRagIndexStatus =
  | "PENDING"
  | "INDEXED"
  | "DISABLED"
  | "WARNING"
  | "FAILED"
  | "STALE";

export interface SkillRagIndexSummary {
  desiredEnabled: boolean;
  sourceVersion: string;
  vectorStatus: SkillRagIndexStatus;
  graphStatus: SkillRagIndexStatus;
  contentHash?: string | null;
  structureHash?: string | null;
  lastIndexedAt?: string | null;
  lastError?: string | null;
  warnings: string[];
}

export interface SkillRagIndexBatchSummary {
  indexedCount: number;
  disabledCount: number;
  failedCount: number;
  warningCount: number;
  warnings: string[];
}

export interface RegistrySkillRecord {
  skillName: string;
  sourceRef: string;
  description?: string | null;
  sourceType: SkillSourceType;
  registryPath: string;
  declaredVersion?: string | null;
  updateStatus: SkillUpdateStatus;
  managedBy: SkillManagedBy;
  lastCheckedAt?: string | null;
  lastUpdatedAt?: string | null;
  lastError?: string | null;
  enabled: boolean;
  sourceKind: SkillWorkspaceSourceKind;
  sourceWorkspaceCode?: string | null;
  sourceWorkspaceName?: string | null;
  sourceLabel?: string | null;
  shadowedWorkspaceCode?: string | null;
  shadowedWorkspaceName?: string | null;
  shadowedLabel?: string | null;
  shadowedEnabled?: boolean | null;
  enabledWorkspaceCount: number;
}

export interface RegistrySkillFile {
  path: string;
  isMarkdown: boolean;
}

export interface RegistrySkillDetail {
  skillName: string;
  sourceRef: string;
  sourceType: SkillSourceType;
  registryPath: string;
  declaredVersion?: string | null;
  updateStatus: SkillUpdateStatus;
  managedBy: SkillManagedBy;
  lastCheckedAt?: string | null;
  lastUpdatedAt?: string | null;
  lastError?: string | null;
  skillMarkdown: string;
  files: RegistrySkillFile[];
  enabled: boolean;
  sourceKind: SkillWorkspaceSourceKind;
  sourceWorkspaceCode?: string | null;
  sourceWorkspaceName?: string | null;
  sourceLabel?: string | null;
  shadowedWorkspaceCode?: string | null;
  shadowedWorkspaceName?: string | null;
  shadowedLabel?: string | null;
  shadowedEnabled?: boolean | null;
  enabledWorkspaceCount: number;
}

export interface WorkspaceSkillRecord {
  skillName: string;
  sourceRef?: string | null;
  sourceType?: SkillSourceType | null;
  registryPath?: string | null;
  declaredVersion?: string | null;
  updateStatus?: SkillUpdateStatus | null;
  managedBy?: SkillManagedBy | null;
  lastCheckedAt?: string | null;
  lastUpdatedAt?: string | null;
  lastError?: string | null;
  enabled: boolean;
  sourceKind: SkillWorkspaceSourceKind;
  sourceWorkspaceCode?: string | null;
  sourceWorkspaceName?: string | null;
  sourceLabel?: string | null;
  shadowedWorkspaceCode?: string | null;
  shadowedWorkspaceName?: string | null;
  shadowedLabel?: string | null;
  shadowedEnabled?: boolean | null;
  registryState: SkillRegistryState;
  ragIndex?: SkillRagIndexSummary | null;
}

export interface WorkspaceSkillDetail {
  skillName: string;
  sourceRef?: string | null;
  sourceType?: SkillSourceType | null;
  registryPath?: string | null;
  declaredVersion?: string | null;
  updateStatus?: SkillUpdateStatus | null;
  managedBy?: SkillManagedBy | null;
  lastCheckedAt?: string | null;
  lastUpdatedAt?: string | null;
  lastError?: string | null;
  skillMarkdown?: string | null;
  files: RegistrySkillFile[];
  enabled: boolean;
  sourceKind: SkillWorkspaceSourceKind;
  sourceWorkspaceCode?: string | null;
  sourceWorkspaceName?: string | null;
  sourceLabel?: string | null;
  shadowedWorkspaceCode?: string | null;
  shadowedWorkspaceName?: string | null;
  shadowedLabel?: string | null;
  shadowedEnabled?: boolean | null;
  registryState: SkillRegistryState;
  ragIndex?: SkillRagIndexSummary | null;
}

export interface SkillSearchCandidate {
  packageRef: string;
  sourceRef: string;
  skillName: string;
  description?: string | null;
  installsLabel?: string | null;
  detailUrl?: string | null;
}

export interface InstallRegistrySkillPayload {
  packageRef?: string;
  sourceRef?: string;
  skillName?: string;
  enableForWorkspaceCode?: string;
}

export interface RegistrySkillMutationResult {
  skillName: string;
  workspaceCode?: string | null;
  enabled: boolean;
  ragIndex?: SkillRagIndexSummary | null;
}

export interface RegistrySkillUninstallResult {
  skillName: string;
  removedBindingsCount: number;
  keptBindingsCount: number;
  ragIndex?: SkillRagIndexBatchSummary | null;
}

export interface SkillBindingMutationResult {
  workspaceCode: string;
  skillName: string;
  enabled: boolean;
  ragIndex?: SkillRagIndexSummary | null;
}

export interface RegistrySkillSyncResult {
  registryRoot: string;
  discoveredCount: number;
  removedCount: number;
  checked: boolean;
  updated: boolean;
  ragIndex?: SkillRagIndexBatchSummary | null;
}
