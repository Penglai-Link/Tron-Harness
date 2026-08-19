import { adaptResourceRecord } from "@/domain/resource/adapters";
import {
  pageResources,
  type ResourcePageApiRequest,
} from "@/domain/resource/api";
import {
  listMockCreatedResources,
  listWorkspaceResources,
} from "@/domain/resource/mock";
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

function toApiRequest(query: ResourcePageQuery, kind: ResourceKind | null): ResourcePageApiRequest {
  return {
    current: query.current,
    kind,
    search: query.search.trim() || null,
    size: query.size,
    workspaceCode: query.workspaceCode ?? "",
  };
}

function mockPage(query: ResourcePageQuery): ResourcePageResult {
  const source = [
    ...listMockCreatedResources(query.workspaceCode ?? ""),
    ...listWorkspaceResources(query.workspaceCode ?? "", query.workspaceName),
  ];
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

export function useResourcePage(query: ResourcePageQuery) {
  return useResourceApiQuery<ResourcePageResult>({
    api: async () => {
      const kinds = query.kinds.length ? query.kinds : [null];
      const pages = await Promise.all(kinds.map(async (kind) => {
        const firstPage = await pageResources(toApiRequest(query, kind));
        const remainingPageCount = Math.ceil(firstPage.total / firstPage.size) - 1;
        const remainingPages = await Promise.all(Array.from({ length: remainingPageCount }, (_, index) => (
          pageResources({ ...toApiRequest(query, kind), current: index + 2 })
        )));
        return [firstPage, ...remainingPages].flatMap((page) => page.records);
      }));
      const records = pages.flat()
        .map(adaptResourceRecord)
        .filter((record) => query.context === "ALL" || record.sourceKind === query.context)
        .sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt));
      const start = (query.current - 1) * query.size;
      return {
        current: query.current,
        records: records.slice(start, start + query.size),
        size: query.size,
        total: records.length,
      };
    },
    deps: [query],
    enabled: Boolean(query.workspaceCode),
    initialData: () => ({ current: query.current, records: [], size: query.size, total: 0 }),
    mock: () => mockPage(query),
  });
}
