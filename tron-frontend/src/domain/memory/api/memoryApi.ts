import { requestApi } from "@/shared/api/httpClient";

import type {
  MemoryRecord,
  MemoryReviewStatus,
  MemoryScopeKind,
  MemoryStatus,
  MemoryType,
} from "@/domain/memory/model";

export const memoryApiEndpoints = {
  listMemoryRecords: "GET /api/v1/memory/workspaces/{workspaceCode}/records",
  createMemoryRecord: "POST /api/v1/memory/workspaces/{workspaceCode}/records",
  getMemoryRecord: "GET /api/v1/memory/workspaces/{workspaceCode}/records/{memoryCode}",
  updateMemoryRecord: "PATCH /api/v1/memory/workspaces/{workspaceCode}/records/{memoryCode}",
  archiveMemoryRecord: "POST /api/v1/memory/workspaces/{workspaceCode}/records/{memoryCode}/archive",
  disableMemoryRecord: "POST /api/v1/memory/workspaces/{workspaceCode}/records/{memoryCode}/disable",
  listMemoryDomains: "GET /api/v1/memory/domains",
  createMemoryDomain: "POST /api/v1/memory/domains",
  updateMemoryDomain: "PATCH /api/v1/memory/domains/{domainCode}",
  enableMemoryDomain: "POST /api/v1/memory/domains/{domainCode}/enable",
  disableMemoryDomain: "POST /api/v1/memory/domains/{domainCode}/disable",
} as const;

export interface MemoryRecordListQuery {
  injectable?: "all" | "injectable" | "not-injectable";
  memoryStatus?: "all" | MemoryStatus;
  memoryType?: MemoryType;
  query?: string;
  reviewStatus?: "all" | MemoryReviewStatus;
  scope: MemoryScopeKind;
  scopeCode: string;
  limit?: number;
}

export interface MemoryMetadataPayload {
  label: string;
  value: string;
}

export interface MemoryMutationPayload {
  injectable: boolean;
  memoryStatus: MemoryStatus;
  memoryType: MemoryType;
  metadata: MemoryMetadataPayload[];
  reviewStatus: MemoryReviewStatus;
  scope: MemoryScopeKind;
  semanticKey: string;
  sourceLabel: string;
  tags: string[];
  targetCode: string;
  text: string;
  title: string;
}

export interface MemoryDeletePayload {
  reason?: string;
}

export interface MemoryWorkDomainRecord {
  createdAt: string;
  description?: string | null;
  domainCode: string;
  name: string;
  status: "ACTIVE" | "DISABLED";
  updatedAt: string;
}

export interface MemoryDomainMutationPayload {
  description?: string | null;
  domainCode?: string;
  name?: string;
  status?: "ACTIVE" | "DISABLED";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown, fallback = "") {
  return typeof value === "string" ? value : fallback;
}

function readBoolean(value: unknown, fallback = false) {
  return typeof value === "boolean" ? value : fallback;
}

function readNumber(value: unknown, fallback = 0) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function normalizeMemoryMetadataItems(value: unknown) {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      if (!isRecord(item)) return null;
      return {
        label: readString(item.label),
        value: readString(item.value),
      };
    })
    .filter((item): item is NonNullable<typeof item> => item !== null);
}

function normalizeMemoryRecord(value: unknown): MemoryRecord | null {
  if (!isRecord(value)) return null;
  const scope = value.scope;
  if (scope !== "space" && scope !== "user" && scope !== "domain") return null;
  return {
    domainCode: typeof value.domainCode === "string" ? value.domainCode : undefined,
    id: readString(value.id),
    injectable: readBoolean(value.injectable),
    memoryCode: readString(value.memoryCode),
    memoryStatus: readString(value.memoryStatus, "ACTIVE") as MemoryStatus,
    memoryType: readString(value.memoryType, "FACT") as MemoryType,
    metadata: normalizeMemoryMetadataItems(value.metadata),
    reviewStatus: readString(value.reviewStatus, "APPROVED") as MemoryReviewStatus,
    scope,
    score: readNumber(value.score, 1),
    semanticKey: readString(value.semanticKey),
    sourceCode: readString(value.sourceCode),
    sourceLabel: readString(value.sourceLabel, "Manual entry"),
    tags: Array.isArray(value.tags)
      ? value.tags.filter((tag): tag is string => typeof tag === "string")
      : [],
    targetCode: readString(value.targetCode),
    targetId: readString(value.targetId),
    targetLabel: readString(value.targetLabel),
    text: readString(value.text),
    title: readString(value.title, "Untitled memory"),
    updatedAt: readString(value.updatedAt),
  };
}

function normalizeMemoryRecords(value: unknown): MemoryRecord[] {
  if (!Array.isArray(value)) return [];
  return value
    .map(normalizeMemoryRecord)
    .filter((item): item is MemoryRecord => item !== null);
}

function requireMemoryRecord(value: unknown, fallbackMessage: string): MemoryRecord {
  const record = normalizeMemoryRecord(value);
  if (!record) {
    throw new Error(fallbackMessage);
  }
  return record;
}

function normalizeMemoryWorkDomainRecord(value: unknown): MemoryWorkDomainRecord | null {
  if (!isRecord(value)) return null;
  const status = value.status;
  if (status !== "ACTIVE" && status !== "DISABLED") return null;
  return {
    createdAt: readString(value.createdAt),
    description: typeof value.description === "string" ? value.description : null,
    domainCode: readString(value.domainCode),
    name: readString(value.name),
    status,
    updatedAt: readString(value.updatedAt),
  };
}

function normalizeMemoryWorkDomains(value: unknown): MemoryWorkDomainRecord[] {
  if (!Array.isArray(value)) return [];
  return value
    .map(normalizeMemoryWorkDomainRecord)
    .filter((item): item is MemoryWorkDomainRecord => item !== null);
}

function mutationBodyFromPayload(payload: MemoryMutationPayload) {
  return {
    injectable: payload.injectable,
    memoryStatus: payload.memoryStatus,
    memoryType: payload.memoryType,
    metadata: payload.metadata,
    reviewStatus: payload.reviewStatus,
    scope: payload.scope,
    scopeCode: payload.targetCode,
    semanticKey: payload.semanticKey,
    sourceLabel: payload.sourceLabel,
    tags: payload.tags,
    text: payload.text,
    title: payload.title,
  };
}

export function fetchMemoryRecords(
  workspaceCode: string,
  query: MemoryRecordListQuery,
  signal?: AbortSignal,
) {
  return requestApi<unknown>(memoryApiEndpoints.listMemoryRecords, {
    pathParams: { workspaceCode },
    queryParams: {
      injectable: query.injectable ?? "all",
      limit: query.limit ?? 50,
      memoryStatus: query.memoryStatus ?? "all",
      memoryType: query.memoryType ?? "",
      query: query.query ?? "",
      reviewStatus: query.reviewStatus ?? "all",
      scope: query.scope,
      scopeCode: query.scopeCode,
    },
    signal,
  }).then(normalizeMemoryRecords);
}

export function fetchMemoryRecord(
  workspaceCode: string,
  memoryCode: string,
  signal?: AbortSignal,
) {
  return requestApi<unknown>(memoryApiEndpoints.getMemoryRecord, {
    pathParams: {
      memoryCode,
      workspaceCode,
    },
    signal,
  }).then((payload) => requireMemoryRecord(payload, "Memory could not be loaded"));
}

export function createMemoryRecord(
  workspaceCode: string,
  payload: MemoryMutationPayload,
) {
  return requestApi<unknown>(memoryApiEndpoints.createMemoryRecord, {
    body: mutationBodyFromPayload(payload),
    pathParams: { workspaceCode },
  }).then((record) => requireMemoryRecord(record, "Memory could not be created"));
}

export function updateMemoryRecord(
  workspaceCode: string,
  memoryCode: string,
  payload: MemoryMutationPayload,
) {
  return requestApi<unknown>(memoryApiEndpoints.updateMemoryRecord, {
    body: mutationBodyFromPayload(payload),
    pathParams: {
      memoryCode,
      workspaceCode,
    },
  }).then((record) => requireMemoryRecord(record, "Memory could not be saved"));
}

export function archiveMemoryRecord(
  workspaceCode: string,
  memoryCode: string,
  payload: MemoryDeletePayload = {},
) {
  return requestApi<unknown>(memoryApiEndpoints.archiveMemoryRecord, {
    body: payload,
    pathParams: {
      memoryCode,
      workspaceCode,
    },
  }).then((record) => requireMemoryRecord(record, "Memory could not be archived"));
}

export function disableMemoryRecord(
  workspaceCode: string,
  memoryCode: string,
  payload: MemoryDeletePayload = {},
) {
  return requestApi<unknown>(memoryApiEndpoints.disableMemoryRecord, {
    body: payload,
    pathParams: {
      memoryCode,
      workspaceCode,
    },
  }).then((record) => requireMemoryRecord(record, "Memory could not be disabled"));
}

export function fetchMemoryDomains(
  query: { includeDisabled?: boolean; search?: string } = {},
  signal?: AbortSignal,
) {
  return requestApi<unknown>(memoryApiEndpoints.listMemoryDomains, {
    queryParams: {
      includeDisabled: query.includeDisabled ?? false,
      search: query.search ?? "",
    },
    signal,
  }).then(normalizeMemoryWorkDomains);
}

export function createMemoryDomain(payload: Required<Pick<MemoryDomainMutationPayload, "domainCode" | "name">> & Pick<MemoryDomainMutationPayload, "description">) {
  return requestApi<unknown>(memoryApiEndpoints.createMemoryDomain, {
    body: payload,
  }).then((domain) => {
    const normalized = normalizeMemoryWorkDomainRecord(domain);
    if (!normalized) throw new Error("Memory domain could not be created");
    return normalized;
  });
}

export function updateMemoryDomain(domainCode: string, payload: Omit<MemoryDomainMutationPayload, "domainCode">) {
  return requestApi<unknown>(memoryApiEndpoints.updateMemoryDomain, {
    body: payload,
    pathParams: { domainCode },
  }).then((domain) => {
    const normalized = normalizeMemoryWorkDomainRecord(domain);
    if (!normalized) throw new Error("Memory domain could not be updated");
    return normalized;
  });
}

export function enableMemoryDomain(domainCode: string) {
  return requestApi<unknown>(memoryApiEndpoints.enableMemoryDomain, {
    pathParams: { domainCode },
  }).then((domain) => {
    const normalized = normalizeMemoryWorkDomainRecord(domain);
    if (!normalized) throw new Error("Memory domain could not be enabled");
    return normalized;
  });
}

export function disableMemoryDomain(domainCode: string) {
  return requestApi<unknown>(memoryApiEndpoints.disableMemoryDomain, {
    pathParams: { domainCode },
  }).then((domain) => {
    const normalized = normalizeMemoryWorkDomainRecord(domain);
    if (!normalized) throw new Error("Memory domain could not be disabled");
    return normalized;
  });
}
