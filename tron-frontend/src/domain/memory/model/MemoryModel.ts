export type MemoryScope = "User" | "Space" | "WorkDomain";
export interface MemoryRecord {
  memoryCode: string;
  scope: MemoryScope;
  scopeCode: string;
  title: string;
  text: string;
  enabled: boolean;
  version: number;
  sourceRefs: Record<string, unknown>[];
  sourceLabel: string;
  expiresAt: number | null;
  createdAt: number;
  updatedAt: number;
  indexStatus: "Ready" | "Pending" | "Failed" | "Cancelled" | "Disabled";
  indexError: string | null;
}
export interface MemoryTarget {
  scope: MemoryScope;
  scopeCode: string;
  label: string;
}
