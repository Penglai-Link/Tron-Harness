import { useMemo } from "react";

import { listWorkspaceResourceVersionDetails } from "@/domain/resource/mock";

export function useResourceVersionDetails(
  workspaceCode: string,
  workspaceName: string,
  resourceCode: string,
) {
  return useMemo(() => (
    listWorkspaceResourceVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.resourceCode === resourceCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt))
  ), [resourceCode, workspaceCode, workspaceName]);
}
