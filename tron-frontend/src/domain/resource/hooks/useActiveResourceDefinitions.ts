import { useMemo } from "react";

import { listWorkspaceResourceDefinitionVersionDetails } from "@/domain/resource/mock";

export function useActiveResourceDefinitions(workspaceCode: string, workspaceName: string) {
  return useMemo(() => (
    listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
      .filter((record) => record.lifecycleStatus === "ACTIVE" && record.enabled)
      .sort((left, right) => left.name.localeCompare(right.name))
  ), [workspaceCode, workspaceName]);
}
