import { adaptResourceDefinitionVersionDetail } from "@/domain/resource/adapters";
import { getResourceDefinition } from "@/domain/resource/api";
import {
  listPlatformResourceDefinitionVersionDetails,
  listWorkspaceResourceDefinitionVersionDetails,
} from "@/domain/resource/mock";
import type { ResourceDefinitionVersionDetail } from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ResourceDefinitionDetailQuery {
  definitionCode: string;
  definitionVersion: string;
  scopeKind: "PLATFORM" | "WORKSPACE";
  workspaceCode: string | null;
  workspaceName: string;
}

export function useResourceDefinitionDetail(query: ResourceDefinitionDetailQuery) {
  return useResourceApiQuery<ResourceDefinitionVersionDetail | null>({
    api: async () => adaptResourceDefinitionVersionDetail(await getResourceDefinition({
      definitionCode: query.definitionCode,
      definitionVersion: query.definitionVersion,
      scopeKind: query.scopeKind,
      workspaceCode: query.scopeKind === "PLATFORM" ? null : query.workspaceCode,
    })),
    deps: [query],
    enabled: Boolean(query.definitionCode && query.definitionVersion)
      && (query.scopeKind === "PLATFORM" || Boolean(query.workspaceCode)),
    initialData: () => null,
    mock: () => {
      const records = query.scopeKind === "PLATFORM"
        ? listPlatformResourceDefinitionVersionDetails()
        : listWorkspaceResourceDefinitionVersionDetails(query.workspaceCode ?? "", query.workspaceName);
      return records.find((record) => (
        record.definitionCode === query.definitionCode
          && record.definitionVersion === query.definitionVersion
      )) ?? null;
    },
  });
}
