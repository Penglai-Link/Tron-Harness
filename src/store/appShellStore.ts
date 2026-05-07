import { create } from "zustand";

import {
  defaultPrimarySpaceId,
  defaultSecondarySpaceId,
} from "@/shared/constants/workspaceSpaces";

interface AppShellState {
  agentOpen: boolean;
  openSections: Record<string, boolean>;
  secondarySpaceMenuOpen: boolean;
  selectedPrimarySpaceId: string;
  selectedSecondarySpaceId: string;
  sidebarCollapsed: boolean;
  utilityPanel: string | null;
  closeFloatingPanels: () => void;
  ensureSectionOpen: (sectionLabel?: string) => void;
  selectPrimarySpace: (primarySpaceId: string, secondarySpaceId: string) => void;
  selectSecondarySpace: (secondarySpaceId: string) => void;
  setAgentOpen: (open: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleAgent: () => void;
  toggleSecondarySpaceMenu: () => void;
  toggleSection: (sectionLabel: string) => void;
  toggleUtilityPanel: (panel: string) => void;
}

export const useAppShellStore = create<AppShellState>((set) => ({
  agentOpen: false,
  openSections: { Core: true },
  secondarySpaceMenuOpen: false,
  selectedPrimarySpaceId: defaultPrimarySpaceId,
  selectedSecondarySpaceId: defaultSecondarySpaceId,
  sidebarCollapsed: false,
  utilityPanel: null,
  closeFloatingPanels: () => set({ secondarySpaceMenuOpen: false, utilityPanel: null }),
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
      secondarySpaceMenuOpen: false,
      selectedPrimarySpaceId: primarySpaceId,
      selectedSecondarySpaceId: secondarySpaceId,
      utilityPanel: null,
    }),
  selectSecondarySpace: (secondarySpaceId) =>
    set({
      secondarySpaceMenuOpen: false,
      selectedSecondarySpaceId: secondarySpaceId,
    }),
  setAgentOpen: (agentOpen) => set({ agentOpen }),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
  toggleAgent: () => set((state) => ({ agentOpen: !state.agentOpen })),
  toggleSecondarySpaceMenu: () =>
    set((state) => ({
      secondarySpaceMenuOpen: !state.secondarySpaceMenuOpen,
      utilityPanel: null,
    })),
  toggleSection: (sectionLabel) =>
    set((state) => ({
      openSections: {
        ...state.openSections,
        [sectionLabel]: !state.openSections[sectionLabel],
      },
    })),
  toggleUtilityPanel: (panel) =>
    set((state) => ({
      secondarySpaceMenuOpen: false,
      utilityPanel: state.utilityPanel === panel ? null : panel,
    })),
}));
