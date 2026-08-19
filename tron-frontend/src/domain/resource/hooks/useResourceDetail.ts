import { adaptResourceRecord } from "@/domain/resource/adapters";
import { getResource } from "@/domain/resource/api";
import { getWorkspaceResourceDetail } from "@/domain/resource/mock";
import type { ResourceRecord } from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ResourceDetailQuery {
  resourceCode: string;
  workspaceCode: string | null;
  workspaceName: string;
}

export function useResourceDetail(query: ResourceDetailQuery) {
  return useResourceApiQuery<ResourceRecord | null>({
    api: async () => adaptResourceRecord(await getResource({
      resourceCode: query.resourceCode,
      workspaceCode: query.workspaceCode ?? "",
    })),
    deps: [query],
    enabled: Boolean(query.resourceCode) && Boolean(query.workspaceCode),
    initialData: () => null,
    mock: () => getWorkspaceResourceDetail(query.workspaceCode ?? "", query.workspaceName, query.resourceCode),
  });
}
