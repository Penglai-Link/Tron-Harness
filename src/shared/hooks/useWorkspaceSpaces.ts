import {
  createContext,
  useContext,
} from "react";

import {
  defaultPrimarySpaceId,
  defaultSecondarySpaceId,
  getSecondarySpaceMocks,
  primarySpaceMocks,
} from "@/shared/constants/workspaceSpaces";
import type { WorkspaceSpacesContextValue } from "@/shared/types/WorkspaceSpace";

const defaultSecondarySpaces = getSecondarySpaceMocks(defaultPrimarySpaceId);

export const WorkspaceSpacesContext = createContext<WorkspaceSpacesContextValue>({
  primarySpaces: primarySpaceMocks,
  secondarySpaces: defaultSecondarySpaces,
  selectedPrimarySpace: primarySpaceMocks.find((space) => space.id === defaultPrimarySpaceId),
  selectedPrimarySpaceId: defaultPrimarySpaceId,
  selectedSecondarySpace: defaultSecondarySpaces.find((space) => space.id === defaultSecondarySpaceId) ?? defaultSecondarySpaces[0],
  selectedSecondarySpaceId: defaultSecondarySpaceId,
});

export function useWorkspaceSpaces() {
  return useContext(WorkspaceSpacesContext);
}
