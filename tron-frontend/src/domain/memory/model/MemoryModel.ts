export type MemoryScopeKind = "space" | "user" | "domain";

export type MemoryType =
  | "FACT"
  | "PROCEDURE"
  | "PROJECT_DECISION"
  | "RISK"
  | "SPACE_RULE"
  | "USER_PREFERENCE"
  | "WORK_DOMAIN_EXPERIENCE";

export type MemoryStatus = "ACTIVE" | "ARCHIVED" | "CANDIDATE" | "DISABLED";

export type MemoryReviewStatus = "APPROVED" | "PENDING_REVIEW" | "REJECTED";

export interface MemorySearchTarget {
  code: string;
  description: string;
  id: string;
  label: string;
  resultCount: number;
  scope: MemoryScopeKind;
}

export interface MemoryMetadataItem {
  label: string;
  value: string;
}

export interface MemoryRecord {
  domainCode?: string;
  id: string;
  injectable: boolean;
  memoryCode: string;
  memoryStatus: MemoryStatus;
  memoryType: MemoryType;
  metadata: MemoryMetadataItem[];
  reviewStatus: MemoryReviewStatus;
  scope: MemoryScopeKind;
  score: number;
  semanticKey: string;
  sourceCode: string;
  sourceLabel: string;
  tags: string[];
  targetCode: string;
  targetId: string;
  targetLabel: string;
  text: string;
  title: string;
  updatedAt: string;
}
