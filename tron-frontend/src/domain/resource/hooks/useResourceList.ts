import { listWorkspaceResourceVersions } from "@/domain/resource/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { resourceApiEndpoints } from "@/domain/resource/api";

export function useResourceList(workspaceCode: string, workspaceName: string) {
  return useServiceQuery({
    apiInitialData: () => [],
    domain: "resource",
    endpoint: resourceApiEndpoints.listResources,
    mockData: () => listWorkspaceResourceVersions(workspaceCode, workspaceName),
    deps: [workspaceCode, workspaceName],
  });
}
