import { adaptResourceDefinitionVersionDetail } from "@/domain/resource/adapters";
import { getResourceDefinition, pageResourceDefinitions } from "@/domain/resource/api";
import { listWorkspaceResourceDefinitionVersionDetails } from "@/domain/resource/mock";
import type { ResourceDefinitionVersionDetail } from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export function useResourceDefinitionVersionDetails(
  workspaceCode: string,
  workspaceName: string,
  definitionCode: string,
) {
  const query = useResourceApiQuery<ResourceDefinitionVersionDetail[]>({
    api: async () => {
      const page = await pageResourceDefinitions({
        current: 1,
        definitionCode,
        definitionStatus: null,
        lifecycleStatus: null,
        scopeKind: "WORKSPACE",
        search: null,
        size: 50,
        sourceKind: null,
        validationStatus: null,
        workspaceCode,
      });
      return Promise.all(page.records.map(async (record) => (
        adaptResourceDefinitionVersionDetail(await getResourceDefinition({
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          scopeKind: "WORKSPACE",
          workspaceCode,
        }))
      )));
    },
    deps: [workspaceCode, workspaceName, definitionCode],
    enabled: Boolean(workspaceCode && definitionCode),
    initialData: () => [],
    mock: () => listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.definitionCode === definitionCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt)),
  });
  return query.data;
}
