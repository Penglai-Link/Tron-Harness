import { adaptResourceVersionRecord } from "@/domain/resource/adapters";
import {
  pageResources,
  type ResourcePageApiRequest,
} from "@/domain/resource/api";
import {
  listPlatformResourceVersions,
  listWorkspaceResourceVersions,
} from "@/domain/resource/mock";
import type {
  ResourceContextFilter,
  ResourceLifecycleFilter,
  ResourceSourceKind,
  ResourceValidationFilter,
  ResourceVersionRecord,
} from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ResourcePageQuery {
  context: ResourceContextFilter;
  current: number;
  definitionCode?: string | null;
  lifecycle: ResourceLifecycleFilter;
  scopeKind: "PLATFORM" | "WORKSPACE";
  search: string;
  size: number;
  validation: ResourceValidationFilter;
  workspaceCode: string | null;
  workspaceName: string;
}

export interface ResourcePageResult {
  current: number;
  records: ResourceVersionRecord[];
  size: number;
  total: number;
}

function sourceFilter(value: ResourceContextFilter): ResourceSourceKind | null {
  if (value === "WORKSPACE") return "LOCAL";
  return value === "PLATFORM" || value === "LOCAL" || value === "INHERITED" || value === "OVERRIDE"
    ? value
    : null;
}

function toApiRequest(query: ResourcePageQuery): ResourcePageApiRequest {
  return {
    current: query.current,
    definitionCode: query.definitionCode?.trim() || null,
    definitionStatus: null,
    lifecycleStatus: query.lifecycle === "ALL" ? null : query.lifecycle,
    resourceCode: null,
    scopeKind: query.scopeKind,
    search: query.search.trim() || null,
    size: query.size,
    sourceKind: sourceFilter(query.context),
    validationStatus: query.validation === "ALL" ? null : query.validation,
    workspaceCode: query.scopeKind === "PLATFORM" ? null : query.workspaceCode,
  };
}

function mockPage(query: ResourcePageQuery): ResourcePageResult {
  const source = query.scopeKind === "PLATFORM"
    ? listPlatformResourceVersions()
    : listWorkspaceResourceVersions(query.workspaceCode ?? "", query.workspaceName);
  const search = query.search.trim().toLowerCase();
  const records = source.filter((record) => {
    if (query.lifecycle !== "ALL" && record.lifecycleStatus !== query.lifecycle) return false;
    if (query.validation !== "ALL" && record.validationStatus !== query.validation) return false;
    if (query.context !== "ALL" && record.sourceKind !== query.context && record.scopeKind !== query.context) return false;
    if (query.definitionCode && record.definitionCode !== query.definitionCode) return false;
    return !search || [record.resourceCode, record.name, record.description]
      .some((value) => value.toLowerCase().includes(search));
  });
  const current = Math.max(1, query.current);
  const start = (current - 1) * query.size;
  return {
    current,
    records: records.slice(start, start + query.size),
    size: query.size,
    total: records.length,
  };
}

export function useResourcePage(query: ResourcePageQuery) {
  return useResourceApiQuery<ResourcePageResult>({
    api: async () => {
      const page = await pageResources(toApiRequest(query));
      return {
        current: page.current,
        records: page.records.map(adaptResourceVersionRecord),
        size: page.size,
        total: page.total,
      };
    },
    deps: [query],
    enabled: query.scopeKind === "PLATFORM" || Boolean(query.workspaceCode),
    initialData: () => ({ current: query.current, records: [], size: query.size, total: 0 }),
    mock: () => mockPage(query),
  });
}
