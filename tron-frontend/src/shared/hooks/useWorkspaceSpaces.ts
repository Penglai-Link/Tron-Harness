import {
  createContext,
  useContext,
} from "react";

import {
  defaultPrimarySpaceId,
  defaultSecondarySpaceId,
  getWorkspaceSwitcherMock,
  getSecondarySpaceMocks,
} from "@/shared/constants/workspaceSpaces";
import type { WorkspaceSpacesContextValue } from "@/shared/types/WorkspaceSpace";

const defaultSwitcher = getWorkspaceSwitcherMock();
const defaultSecondarySpaces = getSecondarySpaceMocks(defaultPrimarySpaceId);

export const WorkspaceSpacesContext = createContext<WorkspaceSpacesContextValue>({
  primarySpaces: defaultSwitcher.primarySpaces,
  secondarySpaces: defaultSecondarySpaces,
  selectedPrimarySpace: defaultSwitcher.primarySpaces.find((space) => space.workspaceCode === defaultPrimarySpaceId),
  selectedPrimarySpaceId: defaultPrimarySpaceId,
  selectedSecondarySpace: defaultSecondarySpaces.find((space) => space.workspaceCode === defaultSecondarySpaceId) ?? defaultSecondarySpaces[0],
  selectedSecondarySpaceId: defaultSecondarySpaceId,
});

export function useWorkspaceSpaces() {
  return useContext(WorkspaceSpacesContext);
}
