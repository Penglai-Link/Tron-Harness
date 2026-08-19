import { useLocation } from "react-router";

import { useWorkspaceSpaces } from "@/shared";

export interface SettingsResourceScope {
  isSettings: boolean;
  workspaceCode: string;
  workspaceName: string;
}

export function useSettingsResourceScope(): SettingsResourceScope {
  const location = useLocation();
  const isSettings = location.pathname.startsWith("/settings/");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const consoleWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  if (isSettings) {
    return {
      isSettings: true,
      workspaceCode: "PLATFORM",
      workspaceName: "Platform",
    };
  }

  return {
    isSettings,
    workspaceCode: consoleWorkspace?.workspaceCode ?? "",
    workspaceName: consoleWorkspace?.name ?? "",
  };
}
