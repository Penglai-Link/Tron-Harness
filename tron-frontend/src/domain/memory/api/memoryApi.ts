import { requestApi } from "@/shared/api/httpClient";
import type { MemoryRecord, MemoryScope } from "../model";
export const memoryApiEndpoints = {
  queryMemoryRecords: "POST /api/v1/memory/records/query",
  createMemoryRecord: "POST /api/v1/memory/records",
  getMemoryRecord: "GET /api/v1/memory/records/{memoryCode}",
  updateMemoryRecord: "PATCH /api/v1/memory/records/{memoryCode}",
  deleteMemoryRecord: "DELETE /api/v1/memory/records/{memoryCode}",
  listMemoryDomains: "GET /api/v1/memory/domains",
  createMemoryDomain: "POST /api/v1/memory/domains",
  updateMemoryDomain: "PATCH /api/v1/memory/domains/{domainCode}",
  enableMemoryDomain: "POST /api/v1/memory/domains/{domainCode}/enable",
  disableMemoryDomain: "POST /api/v1/memory/domains/{domainCode}/disable",
} as const;

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

function normalizeMemoryWorkDomainRecord(
  value: unknown,
): MemoryWorkDomainRecord | null {
  if (!isRecord(value)) return null;
  const status =
    value.status === "Active"
      ? "ACTIVE"
      : value.status === "Disabled"
        ? "DISABLED"
        : undefined;
  if (status !== "ACTIVE" && status !== "DISABLED") return null;
  return {
    createdAt:
      typeof value.createdAt === "number"
        ? new Date(value.createdAt).toISOString()
        : "",
    description:
      typeof value.description === "string" ? value.description : null,
    domainCode: readString(value.domainCode),
    name: readString(value.name),
    status,
    updatedAt:
      typeof value.updatedAt === "number"
        ? new Date(value.updatedAt).toISOString()
        : "",
  };
}

function normalizeMemoryWorkDomains(value: unknown): MemoryWorkDomainRecord[] {
  if (!Array.isArray(value)) return [];
  return value
    .map(normalizeMemoryWorkDomainRecord)
    .filter((item): item is MemoryWorkDomainRecord => item !== null);
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

export function createMemoryDomain(
  payload: Required<Pick<MemoryDomainMutationPayload, "domainCode" | "name">> &
    Pick<MemoryDomainMutationPayload, "description">,
) {
  return requestApi<unknown>(memoryApiEndpoints.createMemoryDomain, {
    body: payload,
  }).then((domain) => {
    const normalized = normalizeMemoryWorkDomainRecord(domain);
    if (!normalized) throw new Error("Memory domain could not be created");
    return normalized;
  });
}

export function updateMemoryDomain(
  domainCode: string,
  payload: Omit<MemoryDomainMutationPayload, "domainCode">,
) {
  return requestApi<unknown>(memoryApiEndpoints.updateMemoryDomain, {
    body: {
      ...payload,
      status:
        payload.status === "ACTIVE"
          ? "Active"
          : payload.status === "DISABLED"
            ? "Disabled"
            : null,
    },
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

export interface MemoryRecordListQuery {
  scope: MemoryScope;
  scopeCode: string;
  keyword: string;
  enabled: boolean | null;
  current: number;
  size: number;
}
export interface MemoryPageData {
  current: number;
  size: number;
  total: number;
  records: MemoryRecord[];
}
export function fetchMemoryRecords(
  query: MemoryRecordListQuery,
  signal?: AbortSignal,
) {
  return requestApi<MemoryPageData>(memoryApiEndpoints.queryMemoryRecords, {
    body: query,
    signal,
  });
}
export function fetchMemoryRecord(memoryCode: string, signal?: AbortSignal) {
  return requestApi<MemoryRecord>(memoryApiEndpoints.getMemoryRecord, {
    pathParams: { memoryCode },
    signal,
  });
}
export function createMemoryRecord(body: {
  scope: MemoryScope;
  scopeCode: string;
  title: string;
  text: string;
}) {
  return requestApi<MemoryRecord>(memoryApiEndpoints.createMemoryRecord, {
    body,
  });
}
export function updateMemoryRecord(
  memoryCode: string,
  body: { version: number; title?: string; text?: string; enabled?: boolean },
) {
  return requestApi<MemoryRecord>(memoryApiEndpoints.updateMemoryRecord, {
    pathParams: { memoryCode },
    body: {
      version: body.version,
      title: body.title ?? null,
      text: body.text ?? null,
      enabled: body.enabled ?? null,
    },
  });
}
export function deleteMemoryRecord(memoryCode: string, version: number) {
  return requestApi<{ memoryCode: string; deleted: boolean }>(
    memoryApiEndpoints.deleteMemoryRecord,
    { pathParams: { memoryCode }, body: { version } },
  );
}
