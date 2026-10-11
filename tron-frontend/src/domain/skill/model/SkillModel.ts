export type SkillSearchMode = "find" | "repo";

export type SkillSourceType =
  | "Marketplace"
  | "GitHub"
  | "Git"
  | "Url"
  | "Local"
  | "Manual"
  | "BuiltIn";

export type SkillUpdateStatus =
  | "Unknown"
  | "UpToDate"
  | "UpdateAvailable"
  | "Manual"
  | "Error";

export type SkillManagedBy = "Cli" | "Manual" | "BuiltIn";

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
  lastIndexedAt?: number | null;
  updatedAt?: number | null;
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
  lastCheckedAt?: number | null;
  lastUpdatedAt?: number | null;
  lastError?: string | null;
  providerPackId?: string | null;
  available: boolean;
  assigned: boolean;
  assignedAgentCount: number;
  disabledReason?: string | null;
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
  lastCheckedAt?: number | null;
  lastUpdatedAt?: number | null;
  lastError?: string | null;
  skillMarkdown: string;
  files: RegistrySkillFile[];
  providerPackId?: string | null;
  available: boolean;
  assigned: boolean;
  assignedAgentCount: number;
  disabledReason?: string | null;
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
}

export interface RegistrySkillMutationResult {
  skillName: string;
  ragIndex?: SkillRagIndexSummary | null;
}

export interface RegistrySkillUninstallResult {
  skillName: string;
  removedAssignmentsCount: number;
  ragIndex?: SkillRagIndexBatchSummary | null;
}

export interface RegistrySkillSyncResult {
  registryRoot: string;
  discoveredCount: number;
  removedCount: number;
  checked: boolean;
  updated: boolean;
  ragIndex?: SkillRagIndexBatchSummary | null;
}
