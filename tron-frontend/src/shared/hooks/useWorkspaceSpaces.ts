import {
  createContext,
  useContext,
} from "react";

import {
  getEmptyWorkspaceSwitcher,
} from "@/shared/constants/workspaceSpaces";
import type { WorkspaceSpacesContextValue } from "@/shared/types/WorkspaceSpace";

const emptySwitcher = getEmptyWorkspaceSwitcher();

export const WorkspaceSpacesContext = createContext<WorkspaceSpacesContextValue>({
  primarySpaces: emptySwitcher.primarySpaces,
  secondarySpaces: emptySwitcher.secondarySpaces,
  selectedPrimarySpace: undefined,
  selectedPrimarySpaceId: "",
  selectedSecondarySpace: undefined,
  selectedSecondarySpaceId: "",
});

export function useWorkspaceSpaces() {
  return useContext(WorkspaceSpacesContext);
}
