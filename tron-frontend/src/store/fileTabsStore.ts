import { create } from "zustand";
import { panelTabId, useTabStore } from "./tabStore";

function activePanelOwner() {
  const state = useTabStore.getState();
  return panelTabId(state.activeTabId, state.workspaceCode);
}

/** 文件面板 tab key 的固定前缀，用于在右侧面板中识别文件 tab。 */
export const FILE_TAB_KEY_PREFIX = "page-files:";

interface FileTabsState {
  /** 保持挂载的文件面板；通过 owner 隔离到各自的工作区标签页。 */
  fileTabKeys: string[];
  fileTabOwners: Record<string, string>;
  activeKeysByOwner: Record<string, string | null>;
  /** 每个文件 tab 的标题（选择文件夹后为文件夹名），未设置时回退默认文案。 */
  fileTabLabels: Record<string, string>;
  openFileTab: (owner?: string) => string;
  closeFileTab: (key: string) => void;
  setActiveFileTab: (key: string | null, owner?: string) => void;
  setFileTabLabel: (key: string, label: string) => void;
}

let nextFileTabId = 1;

export const useFileTabsStore = create<FileTabsState>((set) => ({
  fileTabKeys: [],
  fileTabOwners: {},
  activeKeysByOwner: {},
  fileTabLabels: {},

  openFileTab: (owner = activePanelOwner()) => {
    const key = `${FILE_TAB_KEY_PREFIX}${nextFileTabId++}`;
    set((state) => ({
      fileTabKeys: [...state.fileTabKeys, key],
      fileTabOwners: { ...state.fileTabOwners, [key]: owner },
      activeKeysByOwner: { ...state.activeKeysByOwner, [owner]: key },
    }));
    return key;
  },

  closeFileTab: (key) => {
    set((state) => {
      const fileTabKeys = state.fileTabKeys.filter((item) => item !== key);
      const owner = state.fileTabOwners[key];
      const wasActive = state.activeKeysByOwner[owner] === key;
      const nextActive = wasActive
        ? (fileTabKeys.filter((item) => state.fileTabOwners[item] === owner).at(-1) ?? null)
        : state.activeKeysByOwner[owner] ?? null;
      const fileTabLabels = { ...state.fileTabLabels };
      const fileTabOwners = { ...state.fileTabOwners };
      delete fileTabLabels[key];
      delete fileTabOwners[key];
      return {
        fileTabKeys, fileTabLabels, fileTabOwners,
        activeKeysByOwner: { ...state.activeKeysByOwner, [owner]: nextActive },
      };
    });
  },

  setActiveFileTab: (key, owner = activePanelOwner()) => {
    set((state) => ({ activeKeysByOwner: { ...state.activeKeysByOwner, [owner]: key } }));
  },

  setFileTabLabel: (key, label) => {
    set((state) => {
      if (state.fileTabLabels[key] === label) return state;
      return { fileTabLabels: { ...state.fileTabLabels, [key]: label } };
    });
  },
}));

// Dispose file panels only when their workspace tab closes, never on tab switches.
useTabStore.subscribe((state, previous) => {
  if (state.tabs === previous.tabs && state.workspaceTabs === previous.workspaceTabs) return;
  const owners = new Set(state.getPanelOwnerIds());
  const files = useFileTabsStore.getState();
  for (const key of files.fileTabKeys) {
    if (!owners.has(files.fileTabOwners[key])) files.closeFileTab(key);
  }
});
