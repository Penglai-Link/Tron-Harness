import {
  adaptResourceDefinitionVersionDetail,
  adaptResourceDefinitionVersionRecord,
} from "@/domain/resource/adapters";
import { getResourceDefinition } from "@/domain/resource/api";
import { listWorkspaceResourceDefinitionVersionDetails } from "@/domain/resource/mock";
import type {
  ResourceDefinitionVersionDetail,
  ResourceDefinitionVersionRecord,
} from "@/domain/resource/model";

import { pageAllResourceDefinitions } from "./resourceDefinitionPagination";
import { useResourceApiQuery } from "./useResourceApiQuery";

function loadActiveWorkspaceDefinitions(workspaceCode: string) {
  return pageAllResourceDefinitions({
    definitionCode: null,
    definitionStatus: "ENABLED",
    lifecycleStatus: "ACTIVE",
    scopeKind: "WORKSPACE",
    search: null,
    sourceKind: null,
    validationStatus: null,
    workspaceCode,
  });
}

export function useActiveResourceDefinitions(workspaceCode: string, workspaceName: string) {
  const query = useResourceApiQuery<ResourceDefinitionVersionDetail[]>({
    api: async () => {
      const records = await loadActiveWorkspaceDefinitions(workspaceCode);
      return Promise.all(records.map(async (record) => (
        adaptResourceDefinitionVersionDetail(await getResourceDefinition({
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          scopeKind: "WORKSPACE",
          workspaceCode,
        }))
      )));
    },
    deps: [workspaceCode, workspaceName],
    enabled: Boolean(workspaceCode),
    initialData: () => [],
    mock: () => listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.lifecycleStatus === "ACTIVE" && record.enabled),
  });
  return query.data.slice().sort((left, right) => left.name.localeCompare(right.name));
}

/**
 * A lightweight projection for resource-list definition filtering. Unlike the
 * editor hook above it intentionally avoids one detail request per definition.
 */
export function useActiveResourceDefinitionRecords(workspaceCode: string, workspaceName: string) {
  const query = useResourceApiQuery<ResourceDefinitionVersionRecord[]>({
    api: async () => (await loadActiveWorkspaceDefinitions(workspaceCode))
      .map(adaptResourceDefinitionVersionRecord),
    deps: [workspaceCode, workspaceName],
    enabled: Boolean(workspaceCode),
    initialData: () => [],
    mock: () => listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.lifecycleStatus === "ACTIVE" && record.enabled),
  });
  return {
    ...query,
    data: query.data.slice().sort((left, right) => left.name.localeCompare(right.name)),
  };
}
