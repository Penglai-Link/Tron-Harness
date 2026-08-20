import { adaptResourceRecord } from "@/domain/resource/adapters";
import {
  pageResources,
  toResourceKindCode,
  toResourceSourceCode,
  type ResourcePageApiRequest,
} from "@/domain/resource/api";
import { listWorkspaceResources } from "@/domain/resource/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import type {
  ResourceKind,
  ResourceRecord,
  ResourceSourceKind,
} from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ResourcePageQuery {
  context: ResourceSourceKind | "ALL";
  current: number;
  kinds: readonly ResourceKind[];
  search: string;
  size: number;
  workspaceCode: string | null;
  workspaceName: string;
}

export interface ResourcePageResult {
  current: number;
  records: ResourceRecord[];
  size: number;
  total: number;
}

function toApiRequest(query: ResourcePageQuery): ResourcePageApiRequest {
  return {
    current: query.current,
    kinds: query.kinds.map(toResourceKindCode),
    search: query.search.trim() || null,
    size: query.size,
    sourceKind: query.context === "ALL" ? null : toResourceSourceCode(query.context),
    workspaceCode: query.workspaceCode ?? "",
  };
}

function mockPage(query: ResourcePageQuery): ResourcePageResult {
  const source = listWorkspaceResources(query.workspaceCode ?? "", query.workspaceName);
  const search = query.search.trim().toLowerCase();
  const records = source.filter((record) => {
    if (query.kinds.length && !query.kinds.includes(record.kind)) return false;
    if (query.context !== "ALL" && record.sourceKind !== query.context) return false;
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

export async function queryResourcePage(query: ResourcePageQuery): Promise<ResourcePageResult> {
  if (getServiceDataSource("resource") === "mock") return mockPage(query);

  const page = await pageResources(toApiRequest(query));
  return {
    current: page.current,
    records: page.records.map(adaptResourceRecord),
    size: page.size,
    total: page.total,
  };
}

export function useResourcePage(query: ResourcePageQuery) {
  return useResourceApiQuery<ResourcePageResult>({
    api: () => queryResourcePage(query),
    deps: [query],
    enabled: Boolean(query.workspaceCode),
    initialData: () => ({ current: query.current, records: [], size: query.size, total: 0 }),
    mock: () => mockPage(query),
  });
}
