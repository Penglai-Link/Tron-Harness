import { adaptResourceVersionDetail } from "@/domain/resource/adapters";
import { getResource, pageResources } from "@/domain/resource/api";
import { listWorkspaceResourceVersionDetails } from "@/domain/resource/mock";
import type { ResourceVersionDetail } from "@/domain/resource/model";

import { useResourceApiQuery } from "./useResourceApiQuery";

export function useResourceVersionDetails(
  workspaceCode: string,
  workspaceName: string,
  resourceCode: string,
) {
  const query = useResourceApiQuery<ResourceVersionDetail[]>({
    api: async () => {
      const page = await pageResources({
        current: 1,
        definitionCode: null,
        definitionStatus: null,
        lifecycleStatus: null,
        resourceCode,
        scopeKind: "WORKSPACE",
        search: null,
        size: 50,
        sourceKind: null,
        validationStatus: null,
        workspaceCode,
      });
      return Promise.all(page.records.map(async (record) => (
        adaptResourceVersionDetail(await getResource({
          resourceCode: record.resourceCode,
          resourceVersion: record.resourceVersion,
          scopeKind: "WORKSPACE",
          workspaceCode,
        }))
      )));
    },
    deps: [workspaceCode, workspaceName, resourceCode],
    enabled: Boolean(workspaceCode && resourceCode),
    initialData: () => [],
    mock: () => listWorkspaceResourceVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.resourceCode === resourceCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt)),
  });
  return query.data;
}
