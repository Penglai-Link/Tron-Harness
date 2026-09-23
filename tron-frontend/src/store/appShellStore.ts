import { create } from "zustand";
import {
  createJSONStorage,
  persist,
} from "zustand/middleware";

import {
  getEmptyWorkspaceSwitcher,
} from "@/shared/constants/workspaceSpaces";
import {
  createDefaultShellShortcutBindings,
  migrateDefaultShellShortcutBindings,
  type ShellShortcutAction,
  type ShellShortcutBinding,
  type ShellShortcutBindings,
} from "@/shared/lib/keyboardShortcuts";
import type { WorkspaceSwitcherData } from "@/shared/types/WorkspaceSpace";

interface AppShellState {
  agentOpen: boolean;
  openSections: Record<string, boolean>;
  selectedPrimarySpaceId: string;
  selectedSecondarySpaceId: string;
  rightSidebarCollapsed: boolean;
  shortcutBindings: ShellShortcutBindings;
  sidebarCollapsed: boolean;
  utilityPanel: string | null;
  workspaceSwitcherSnapshot: WorkspaceSwitcherData;
  closeFloatingPanels: () => void;
  ensureSectionOpen: (sectionLabel?: string) => void;
  selectPrimarySpace: (primarySpaceId: string, secondarySpaceId: string) => void;
  syncSelectedWorkspace: (primarySpaceId: string, secondarySpaceId: string) => void;
  setAgentOpen: (open: boolean) => void;
  setRightSidebarCollapsed: (collapsed: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setShortcutBinding: (
    action: ShellShortcutAction,
    binding: ShellShortcutBinding | null,
  ) => void;
  setWorkspaceSwitcherSnapshot: (snapshot: WorkspaceSwitcherData) => void;
  toggleAgent: () => void;
  toggleSection: (sectionLabel: string) => void;
  toggleUtilityPanel: (panel: string) => void;
}

const defaultWorkspaceSwitcherSnapshot = getEmptyWorkspaceSwitcher();

function migrateAppShellState(persistedState: unknown) {
  if (!persistedState || typeof persistedState !== "object") {
    return persistedState;
  }
  const state = persistedState as Partial<AppShellState>;

  return {
    ...state,
    rightSidebarCollapsed: true,
    selectedPrimarySpaceId: "",
    selectedSecondarySpaceId: "",
    shortcutBindings: migrateDefaultShellShortcutBindings(state.shortcutBindings),
    workspaceSwitcherSnapshot: defaultWorkspaceSwitcherSnapshot,
  };
}

export const useAppShellStore = create<AppShellState>()(
  persist(
    (set) => ({
      agentOpen: false,
      openSections: { Core: true },
      selectedPrimarySpaceId: "",
      selectedSecondarySpaceId: "",
      rightSidebarCollapsed: true,
      shortcutBindings: createDefaultShellShortcutBindings(),
      sidebarCollapsed: false,
      utilityPanel: null,
      workspaceSwitcherSnapshot: defaultWorkspaceSwitcherSnapshot,
      closeFloatingPanels: () => set({ agentOpen: false, utilityPanel: null }),
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
      setRightSidebarCollapsed: (rightSidebarCollapsed) => set({ rightSidebarCollapsed }),
      setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
      setShortcutBinding: (action, binding) =>
        set((state) => ({
          shortcutBindings: {
            ...state.shortcutBindings,
            [action]: binding,
          },
        })),
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
      version: 8,
      name: "tron-app-shell",
      migrate: migrateAppShellState,
      partialize: (state) => ({
        openSections: state.openSections,
        selectedPrimarySpaceId: state.selectedPrimarySpaceId,
        selectedSecondarySpaceId: state.selectedSecondarySpaceId,
        sidebarCollapsed: state.sidebarCollapsed,
        shortcutBindings: state.shortcutBindings,
        workspaceSwitcherSnapshot: state.workspaceSwitcherSnapshot,
      }),
      storage: createJSONStorage(() => localStorage),
    },
  ),
);
