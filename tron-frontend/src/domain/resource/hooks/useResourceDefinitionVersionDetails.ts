import { useMemo } from "react";

import { listWorkspaceResourceDefinitionVersionDetails } from "@/domain/resource/mock";

export function useResourceDefinitionVersionDetails(
  workspaceCode: string,
  workspaceName: string,
  definitionCode: string,
) {
  return useMemo(() => (
    listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.definitionCode === definitionCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt))
  ), [definitionCode, workspaceCode, workspaceName]);
}
