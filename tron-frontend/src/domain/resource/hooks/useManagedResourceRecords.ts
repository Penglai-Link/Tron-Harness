import { useMemo } from "react";

import {
  fromResourceKindCode,
  fromResourceSourceCode,
  pageConnections,
  toResourceKindCode,
} from "@/domain/resource/api";
import { listWorkspaceResources } from "@/domain/resource/mock";
import type { ResourceApiRecord } from "@/domain/resource/api";
import type { ResourceKind, ResourceSourceKind } from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ManagedResourceRecord {
  workspaceCode: string;
  attributes: Record<string, unknown>;
  description: string;
  kind: ResourceKind;
  name: string;
  resourceCode: string;
  sourceKind: ResourceSourceKind;
  updatedAt: number;
}

interface ManagedResourceQuery {
  kinds: readonly ResourceKind[];
  search: string;
  workspaceCode: string | null;
  workspaceName: string;
}

function toManagedRecord(record: ResourceApiRecord): ManagedResourceRecord {
  return {
    workspaceCode: record.workspaceCode,
    attributes: record.attributes ?? {},
    description: record.description,
    kind: fromResourceKindCode(record.kind),
    name: record.name,
    resourceCode: record.resourceCode,
    sourceKind: fromResourceSourceCode(record.sourceKind),
    updatedAt: record.updatedAt,
  };
}

function mockRecords(query: ManagedResourceQuery): ManagedResourceRecord[] {
  const search = query.search.trim().toLowerCase();
  return listWorkspaceResources(query.workspaceCode ?? "", query.workspaceName)
    .filter((record) => query.kinds.includes(record.kind))
    .filter((record) => !search || [record.name, record.resourceCode, record.description]
      .some((value) => value.toLowerCase().includes(search)))
    .map((record) => ({
      workspaceCode: record.workspaceCode,
    attributes: record.attributes ?? {},
      description: record.description,
      kind: record.kind,
      name: record.name,
      resourceCode: record.resourceCode,
      sourceKind: record.sourceKind,
      updatedAt: Date.parse(record.updatedAt),
    }));
}

export function useManagedResourceRecords(query: ManagedResourceQuery) {
  const kindKey = query.kinds.join("|");
  const result = useResourceApiQuery<ManagedResourceRecord[]>({
    api: async () => {
      const request = {
        current: 1,
        kinds: query.kinds.map(toResourceKindCode),
        search: query.search.trim() || null,
        size: 50,
        workspaceCode: query.workspaceCode ?? "",
      };
      const firstPage = await pageConnections(request);
      const remainingPages = await Promise.all(Array.from(
        { length: Math.max(0, Math.ceil(firstPage.total / firstPage.size) - 1) },
        (_, index) => pageConnections({ ...request, current: index + 2 }),
      ));
      return [firstPage, ...remainingPages]
        .flatMap((page) => page.records.map(toManagedRecord))
        .sort((left, right) => right.updatedAt - left.updatedAt);
    },
    deps: [kindKey, query.search, query.workspaceCode],
    enabled: Boolean(query.workspaceCode),
    initialData: () => [],
    mock: () => mockRecords(query),
  });

  return useMemo(() => result, [result]);
}
