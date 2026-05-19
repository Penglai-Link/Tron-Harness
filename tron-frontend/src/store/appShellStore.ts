import { create } from "zustand";

import {
  defaultPrimarySpaceId,
  defaultSecondarySpaceId,
} from "@/shared/constants/workspaceSpaces";

interface AppShellState {
  agentOpen: boolean;
  openSections: Record<string, boolean>;
  selectedPrimarySpaceId: string;
  selectedSecondarySpaceId: string;
  sidebarCollapsed: boolean;
  utilityPanel: string | null;
  closeFloatingPanels: () => void;
  ensureSectionOpen: (sectionLabel?: string) => void;
  selectPrimarySpace: (primarySpaceId: string, secondarySpaceId: string) => void;
  setAgentOpen: (open: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleAgent: () => void;
  toggleSection: (sectionLabel: string) => void;
  toggleUtilityPanel: (panel: string) => void;
}

export const useAppShellStore = create<AppShellState>((set) => ({
  agentOpen: false,
  openSections: { Core: true },
  selectedPrimarySpaceId: defaultPrimarySpaceId,
  selectedSecondarySpaceId: defaultSecondarySpaceId,
  sidebarCollapsed: false,
  utilityPanel: null,
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
  setAgentOpen: (agentOpen) => set({ agentOpen }),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
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
}));
