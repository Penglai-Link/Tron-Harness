import { adaptResourceVersionDetail } from "@/domain/resource/adapters";
import { getResource } from "@/domain/resource/api";
import {
  listPlatformResourceVersionDetails,
  listWorkspaceResourceVersionDetails,
} from "@/domain/resource/mock";
import type { ResourceVersionDetail } from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ResourceDetailQuery {
  resourceCode: string;
  resourceVersion: string;
  scopeKind: "PLATFORM" | "WORKSPACE";
  workspaceCode: string | null;
  workspaceName: string;
}

export function useResourceDetail(query: ResourceDetailQuery) {
  return useResourceApiQuery<ResourceVersionDetail | null>({
    api: async () => adaptResourceVersionDetail(await getResource({
      resourceCode: query.resourceCode,
      resourceVersion: query.resourceVersion,
      scopeKind: query.scopeKind,
      workspaceCode: query.scopeKind === "PLATFORM" ? null : query.workspaceCode,
    })),
    deps: [query],
    enabled: Boolean(query.resourceCode && query.resourceVersion)
      && (query.scopeKind === "PLATFORM" || Boolean(query.workspaceCode)),
    initialData: () => null,
    mock: () => {
      const records = query.scopeKind === "PLATFORM"
        ? listPlatformResourceVersionDetails()
        : listWorkspaceResourceVersionDetails(query.workspaceCode ?? "", query.workspaceName);
      return records.find((record) => (
        record.resourceCode === query.resourceCode
          && record.resourceVersion === query.resourceVersion
      )) ?? null;
    },
  });
}
