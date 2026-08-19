import { useMemo } from "react";

import { pageConnections } from "@/domain/resource/api";
import { listWorkspaceResources } from "@/domain/resource/mock";
import type {
  ResourceApiRecord,
  ResourceKindCode,
  ResourceSourceCode,
} from "@/domain/resource/api";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ManagedResourceRecord {
  attributes: Record<string, unknown>;
  description: string;
  kind: ResourceKindCode;
  name: string;
  resourceCode: string;
  sourceKind: ResourceSourceCode;
  updatedAt: number;
}

interface ManagedResourceQuery {
  kinds: readonly ResourceKindCode[];
  search: string;
  workspaceCode: string | null;
  workspaceName: string;
}

function toManagedRecord(record: ResourceApiRecord): ManagedResourceRecord {
  return {
    attributes: record.attributes ?? {},
    description: record.description,
    kind: record.kind,
    name: record.name,
    resourceCode: record.resourceCode,
    sourceKind: record.sourceKind,
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
      const pages = await Promise.all(query.kinds.map((kind) => pageConnections({
        current: 1,
        kind,
        search: query.search.trim() || null,
        size: 200,
        workspaceCode: query.workspaceCode ?? "",
      })));
      return pages
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
