import { create } from "zustand";
import {
  createJSONStorage,
  persist,
} from "zustand/middleware";

import {
  defaultPrimarySpaceId,
  defaultSecondarySpaceId,
  getEmptyWorkspaceSwitcher,
} from "@/shared/constants/workspaceSpaces";
import type { WorkspaceSwitcherData } from "@/shared/types/WorkspaceSpace";

interface AppShellState {
  agentOpen: boolean;
  openSections: Record<string, boolean>;
  selectedPrimarySpaceId: string;
  selectedSecondarySpaceId: string;
  sidebarCollapsed: boolean;
  utilityPanel: string | null;
  workspaceSwitcherSnapshot: WorkspaceSwitcherData;
  closeFloatingPanels: () => void;
  ensureSectionOpen: (sectionLabel?: string) => void;
  selectPrimarySpace: (primarySpaceId: string, secondarySpaceId: string) => void;
  syncSelectedWorkspace: (primarySpaceId: string, secondarySpaceId: string) => void;
  setAgentOpen: (open: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setWorkspaceSwitcherSnapshot: (snapshot: WorkspaceSwitcherData) => void;
  toggleAgent: () => void;
  toggleSection: (sectionLabel: string) => void;
  toggleUtilityPanel: (panel: string) => void;
}

const defaultWorkspaceSwitcherSnapshot = getEmptyWorkspaceSwitcher();

export const useAppShellStore = create<AppShellState>()(
  persist(
    (set) => ({
      agentOpen: false,
      openSections: { Core: true },
      selectedPrimarySpaceId: defaultPrimarySpaceId,
      selectedSecondarySpaceId: defaultSecondarySpaceId,
      sidebarCollapsed: false,
      utilityPanel: null,
      workspaceSwitcherSnapshot: defaultWorkspaceSwitcherSnapshot,
      closeFloatingPanels: () => set({ utilityPanel: null }),
      ensureSectionOpen: (sectionLabel) =>
        set((state) => {
          if (!sectionLabel || state.openSections[sectionLabel]) {
            return state;
          }

          return {
            openSections: {
              ...state.openSections,
              [sectionLabel]: true,
            },
          };
        }),
      selectPrimarySpace: (primarySpaceId, secondarySpaceId) =>
        set({
          selectedPrimarySpaceId: primarySpaceId,
          selectedSecondarySpaceId: secondarySpaceId,
          utilityPanel: null,
        }),
      syncSelectedWorkspace: (primarySpaceId, secondarySpaceId) =>
        set({
          selectedPrimarySpaceId: primarySpaceId,
          selectedSecondarySpaceId: secondarySpaceId,
        }),
      setAgentOpen: (agentOpen) => set({ agentOpen }),
      setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
      setWorkspaceSwitcherSnapshot: (workspaceSwitcherSnapshot) => set({ workspaceSwitcherSnapshot }),
      toggleAgent: () => set((state) => ({ agentOpen: !state.agentOpen })),
      toggleSection: (sectionLabel) =>
        set((state) => ({
          openSections: {
            ...state.openSections,
            [sectionLabel]: !state.openSections[sectionLabel],
          },
        })),
      toggleUtilityPanel: (panel) =>
        set((state) => ({
          utilityPanel: state.utilityPanel === panel ? null : panel,
        })),
    }),
    {
      name: "tron-app-shell",
      partialize: (state) => ({
        openSections: state.openSections,
        selectedPrimarySpaceId: state.selectedPrimarySpaceId,
        selectedSecondarySpaceId: state.selectedSecondarySpaceId,
        sidebarCollapsed: state.sidebarCollapsed,
        workspaceSwitcherSnapshot: state.workspaceSwitcherSnapshot,
      }),
      storage: createJSONStorage(() => localStorage),
    },
  ),
);
