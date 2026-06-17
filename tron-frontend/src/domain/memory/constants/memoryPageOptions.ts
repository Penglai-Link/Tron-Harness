import type {
  MemoryReviewStatus,
  MemoryStatus,
} from "@/domain/memory/model";

export type MemoryInjectableFilter = "all" | "injectable" | "not-injectable";
export type MemoryReviewFilter = "all" | MemoryReviewStatus;
export type MemoryStatusFilter = "all" | MemoryStatus;

export interface MemoryFilterState {
  injectable: MemoryInjectableFilter;
  reviewStatus: MemoryReviewFilter;
  status: MemoryStatusFilter;
}

export const defaultMemoryFilters: MemoryFilterState = {
  injectable: "all",
  reviewStatus: "all",
  status: "ACTIVE",
};

export const memoryStatusOptions = [
  { label: "Any status", value: "all" },
  { label: "Active", value: "ACTIVE" },
  { label: "Candidate", value: "CANDIDATE" },
  { label: "Archived", value: "ARCHIVED" },
  { label: "Disabled", value: "DISABLED" },
] as const;

export const memoryReviewOptions = [
  { label: "Any review", value: "all" },
  { label: "Approved", value: "APPROVED" },
  { label: "Pending review", value: "PENDING_REVIEW" },
  { label: "Rejected", value: "REJECTED" },
] as const;

export const memoryInjectableOptions = [
  { label: "Any use", value: "all" },
  { label: "Auto context", value: "injectable" },
  { label: "Search only", value: "not-injectable" },
] as const;
