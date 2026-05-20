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
}

export interface RegistrySkillUninstallResult {
  skillName: string;
  removedBindingsCount: number;
  keptBindingsCount: number;
}

export interface SkillBindingMutationResult {
  workspaceCode: string;
  skillName: string;
  enabled: boolean;
}

export interface RegistrySkillSyncResult {
  registryRoot: string;
  discoveredCount: number;
  removedCount: number;
  checked: boolean;
  updated: boolean;
}
