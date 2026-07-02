import { create } from "zustand";

export interface TabItem {
  id: string;
  title: string;
  path: string;
  closable: boolean;
  activeNav?: string;
}

interface TabState {
  tabs: TabItem[];
  activeTabId: string | null;
  addTab: (tab: Omit<TabItem, "id">) => string;
  removeTab: (tabId: string) => void;
  setActiveTab: (tabId: string) => void;
  updateTab: (tabId: string, updates: Partial<Omit<TabItem, "id">>) => void;
  clearAllTabs: () => void;
  getTabByPath: (path: string) => TabItem | undefined;
  getTabByActiveNav: (activeNav: string) => TabItem | undefined;
}

function generateTabId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export const useTabStore = create<TabState>()((set, get) => ({
  tabs: [],
  activeTabId: null,

  addTab: (tab) => {
    // 如果有 activeNav，优先根据 activeNav 查找已存在的 tab
    if (tab.activeNav) {
      const existingTabByNav = get().tabs.find(t => t.activeNav === tab.activeNav);
      if (existingTabByNav) {
        // 更新现有 tab 的 path 和 title
        set((state) => ({
          tabs: state.tabs.map(t =>
            t.id === existingTabByNav.id
              ? { ...t, path: tab.path, title: tab.title }
              : t
          ),
          activeTabId: existingTabByNav.id,
        }));
        return existingTabByNav.id;
      }
    }

    // 否则按原逻辑根据 path 查找
    const existingTab = get().tabs.find(t => t.path === tab.path);

    if (existingTab) {
      set({ activeTabId: existingTab.id });
      return existingTab.id;
    }

    const newTab: TabItem = {
      ...tab,
      id: generateTabId(),
    };

    set((state) => ({
      tabs: [...state.tabs, newTab],
      activeTabId: newTab.id,
    }));

    return newTab.id;
  },

  removeTab: (tabId) => {
    set((state) => {
      const tabs = state.tabs.filter(t => t.id !== tabId);
      let nextActiveTabId = state.activeTabId;

      if (state.activeTabId === tabId) {
        if (tabs.length > 0) {
          const removedIndex = state.tabs.findIndex(t => t.id === tabId);
          const nextIndex = Math.min(removedIndex, tabs.length - 1);
          nextActiveTabId = tabs[nextIndex]?.id || null;
        } else {
          nextActiveTabId = null;
        }
      }

      return {
        tabs,
        activeTabId: nextActiveTabId,
      };
    });
  },

  setActiveTab: (tabId) => {
    const tab = get().tabs.find(t => t.id === tabId);
    if (tab) {
      set({ activeTabId: tabId });
    }
  },

  updateTab: (tabId, updates) => {
    set((state) => ({
      tabs: state.tabs.map(t =>
        t.id === tabId ? { ...t, ...updates } : t
      ),
    }));
  },

  clearAllTabs: () => {
    set({ tabs: [], activeTabId: null });
  },

  getTabByPath: (path) => {
    return get().tabs.find(t => t.path === path);
  },

  getTabByActiveNav: (activeNav) => {
    return get().tabs.find(t => t.activeNav === activeNav);
  },
}));
