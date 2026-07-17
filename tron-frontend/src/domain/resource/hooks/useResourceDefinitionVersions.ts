import { useMemo } from "react";

import { listWorkspaceResourceDefinitionVersions } from "@/domain/resource/mock/resourceDefinitionData";

/**
 * UI-only data boundary. Backend integration can replace the implementation
 * without changing the resource definition pages.
 */
export function useResourceDefinitionVersions(workspaceCode: string, workspaceName: string) {
  return useMemo(
    () => listWorkspaceResourceDefinitionVersions(workspaceCode, workspaceName),
    [workspaceCode, workspaceName],
  );
}
