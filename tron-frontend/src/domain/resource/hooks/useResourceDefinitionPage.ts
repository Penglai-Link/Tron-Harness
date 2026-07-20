import { adaptResourceDefinitionVersionRecord } from "@/domain/resource/adapters";
import {
  pageResourceDefinitions,
  type ResourceDefinitionPageApiRequest,
} from "@/domain/resource/api";
import {
  listPlatformResourceDefinitionVersions,
  listWorkspaceResourceDefinitionVersions,
} from "@/domain/resource/mock";
import type {
  ResourceDefinitionEnabledFilter,
  ResourceDefinitionLifecycleFilter,
  ResourceDefinitionSourceKind,
  ResourceDefinitionValidationFilter,
  ResourceDefinitionVersionRecord,
} from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ResourceDefinitionPageQuery {
  current: number;
  enabled: ResourceDefinitionEnabledFilter;
  lifecycle: ResourceDefinitionLifecycleFilter;
  scopeKind: "PLATFORM" | "WORKSPACE";
  search: string;
  size: number;
  sourceKind?: ResourceDefinitionSourceKind | "ALL";
  validation: ResourceDefinitionValidationFilter;
  workspaceCode: string | null;
  workspaceName: string;
}

export interface ResourceDefinitionPageResult {
  current: number;
  records: ResourceDefinitionVersionRecord[];
  size: number;
  total: number;
}

function toApiRequest(query: ResourceDefinitionPageQuery): ResourceDefinitionPageApiRequest {
  return {
    current: query.current,
    definitionCode: null,
    definitionStatus: query.enabled === "ALL" ? null : query.enabled,
    lifecycleStatus: query.lifecycle === "ALL" ? null : query.lifecycle,
    scopeKind: query.scopeKind,
    search: query.search.trim() || null,
    size: query.size,
    sourceKind: query.sourceKind && query.sourceKind !== "ALL" ? query.sourceKind : null,
    validationStatus: query.validation === "ALL" ? null : query.validation,
    workspaceCode: query.scopeKind === "PLATFORM" ? null : query.workspaceCode,
  };
}

function mockPage(query: ResourceDefinitionPageQuery): ResourceDefinitionPageResult {
  const source = query.scopeKind === "PLATFORM"
    ? listPlatformResourceDefinitionVersions()
    : listWorkspaceResourceDefinitionVersions(query.workspaceCode ?? "", query.workspaceName);
  const search = query.search.trim().toLowerCase();
  const records = source.filter((record) => {
    if (query.lifecycle !== "ALL" && record.lifecycleStatus !== query.lifecycle) return false;
    if (query.enabled !== "ALL" && (query.enabled === "ENABLED") !== record.enabled) return false;
    if (query.validation !== "ALL" && record.validationStatus !== query.validation) return false;
    if (query.sourceKind && query.sourceKind !== "ALL" && record.sourceKind !== query.sourceKind) return false;
    return !search || [record.definitionCode, record.name, record.description]
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

export function useResourceDefinitionPage(query: ResourceDefinitionPageQuery) {
  return useResourceApiQuery<ResourceDefinitionPageResult>({
    api: async () => {
      const page = await pageResourceDefinitions(toApiRequest(query));
      return {
        current: page.current,
        records: page.records.map(adaptResourceDefinitionVersionRecord),
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
