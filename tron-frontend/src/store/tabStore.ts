import { create } from "zustand";

export interface TabItem {
  id: string;
  title: string;
  titleTranslationKey?: string;
  path: string;
  closable: boolean;
  activeNav?: string;
  rightPanelOpen?: boolean;
  rightPanelSize?: number;
}

export interface TabRightPanelState {
  open?: boolean;
  size?: number;
}

/**
 * /shared-shell-preview is not a console route, so no tab is auto-created
 * for it and the sidebar stays fully clickable afterwards.
 */
export const DEFAULT_TAB_CLOSE_REDIRECT_PATH = "/shared-shell-preview";

// 缓存的 tab 信息
export interface CachedTabInfo {
  tabId: string;
  path: string;
  pageId: string;
  lastAccessTime: number;
  revision: number;
}

interface TabState {
  tabs: TabItem[];
  activeTabId: string | null;
  cachedTabsMap: Map<string, CachedTabInfo>; // key: tabId, value: 缓存信息
  cachedTabsList: CachedTabInfo[]; // 缓存的 tab 列表，用于渲染
  maxCachedTabs: number; // 最大缓存数量
  addTab: (tab: Omit<TabItem, "id">) => string;
  closeTab: (tabId: string) => string | null;
  removeTab: (tabId: string) => void;
  removeTabs: (tabIds: readonly string[]) => void;
  refreshTab: (tabId: string) => void;
  setActiveTab: (tabId: string) => void;
  updateRightPanelState: (tabId: string, panelState: TabRightPanelState) => void;
  updateTab: (tabId: string, updates: Partial<Omit<TabItem, "id">>) => void;
  clearAllTabs: () => void;
  getTabByPath: (path: string) => TabItem | undefined;
  getTabByActiveNav: (activeNav: string) => TabItem | undefined;
  addToCache: (tabId: string, path: string, pageId: string) => void;
  removeFromCache: (tabId: string) => void;
  isTabCached: (tabId: string) => boolean;
}

function generateTabId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export const useTabStore = create<TabState>()((set, get) => ({
  tabs: [],
  activeTabId: null,
  cachedTabsMap: new Map(),
  cachedTabsList: [],
  maxCachedTabs: 10, // 最多缓存 10 个 tab

  addTab: (tab) => {
    const state = get();

    // 首先根据 path 精确查找
    const existingTabByPath = state.tabs.find(t => t.path === tab.path);
    if (existingTabByPath) {
      // 找到了精确匹配的 tab，只更新 activeTabId
      if (state.activeTabId !== existingTabByPath.id) {
        set({ activeTabId: existingTabByPath.id });
      }
      return existingTabByPath.id;
    }

    // 如果有 activeNav，根据 activeNav 查找已存在的 tab
    if (tab.activeNav) {
      const existingTabByNav = state.tabs.find(t => t.activeNav === tab.activeNav);
      if (existingTabByNav) {
        // 找到了同 activeNav 的 tab，返回它的 ID
        // 不在这里更新 activeTabId，让外部调用者决定
        return existingTabByNav.id;
      }
    }

    // 创建新 tab
    const newTab: TabItem = {
      ...tab,
      id: generateTabId(),
      rightPanelOpen: tab.rightPanelOpen ?? false,
      rightPanelSize: tab.rightPanelSize ?? 520,
    };

    set({
      tabs: [...state.tabs, newTab],
      activeTabId: newTab.id,
    });

    return newTab.id;
  },

  /**
   * Closes a closable tab. Returns the path the caller should navigate to
   * when the closed tab was the active one, or null when there is nothing
   * to navigate (tab not closable, or an inactive tab was closed).
   */
  closeTab: (tabId) => {
    const state = get();
    const target = state.tabs.find((tab) => tab.id === tabId);
    if (!target?.closable) return null;

    const wasActive = state.activeTabId === tabId;
    get().removeTab(tabId);
    if (!wasActive) return null;

    const nextActiveTab = get().tabs.find((tab) => tab.id === get().activeTabId);
    return nextActiveTab?.path ?? DEFAULT_TAB_CLOSE_REDIRECT_PATH;
  },

  removeTab: (tabId) => get().removeTabs([tabId]),

  removeTabs: (tabIds) => {
    const tabIdSet = new Set(tabIds);
    if (tabIdSet.size === 0) return;

    set((state) => {
      const tabs = state.tabs.filter((tab) => !tabIdSet.has(tab.id));
      let nextActiveTabId = state.activeTabId;

      if (state.activeTabId && tabIdSet.has(state.activeTabId)) {
        if (tabs.length > 0) {
          const removedIndex = state.tabs.findIndex((tab) => tab.id === state.activeTabId);
          const nextIndex = Math.min(removedIndex, tabs.length - 1);
          nextActiveTabId = tabs[nextIndex]?.id || null;
        } else {
          nextActiveTabId = null;
        }
      }

      // 从缓存中移除
      const newCachedTabsMap = new Map(state.cachedTabsMap);
      tabIdSet.forEach((tabId) => newCachedTabsMap.delete(tabId));
      const newCachedTabsList = Array.from(newCachedTabsMap.values());

      return {
        tabs,
        activeTabId: nextActiveTabId,
        cachedTabsMap: newCachedTabsMap,
        cachedTabsList: newCachedTabsList,
      };
    });
  },

  refreshTab: (tabId) => {
    set((state) => {
      const cachedTab = state.cachedTabsMap.get(tabId);
      if (!cachedTab) return state;

      const cachedTabsMap = new Map(state.cachedTabsMap);
      cachedTabsMap.set(tabId, {
        ...cachedTab,
        lastAccessTime: Date.now(),
        revision: cachedTab.revision + 1,
      });
      return {
        cachedTabsMap,
        cachedTabsList: Array.from(cachedTabsMap.values()),
      };
    });
  },

  setActiveTab: (tabId) => {
    const tab = get().tabs.find(t => t.id === tabId);
    if (tab) {
      set({ activeTabId: tabId });
    }
  },

  updateRightPanelState: (tabId, panelState) => {
    set((state) => {
      let changed = false;
      const tabs = state.tabs.map((tab) => {
        if (tab.id !== tabId) return tab;

        const nextOpen = panelState.open ?? tab.rightPanelOpen;
        const nextSize = panelState.size ?? tab.rightPanelSize;
        if (nextOpen === tab.rightPanelOpen && nextSize === tab.rightPanelSize) {
          return tab;
        }

        changed = true;
        return {
          ...tab,
          rightPanelOpen: nextOpen,
          rightPanelSize: nextSize,
        };
      });

      return changed ? { tabs } : state;
    });
  },

  updateTab: (tabId, updates) => {
    set((state) => ({
      tabs: state.tabs.map(t =>
        t.id === tabId ? { ...t, ...updates } : t
      ),
    }));
  },

  clearAllTabs: () => {
    set({
      tabs: [],
      activeTabId: null,
      cachedTabsMap: new Map(),
      cachedTabsList: [],
    });
  },

  getTabByPath: (path) => {
    return get().tabs.find(t => t.path === path);
  },

  getTabByActiveNav: (activeNav) => {
    return get().tabs.find(t => t.activeNav === activeNav);
  },

  addToCache: (tabId, path, pageId) => {
    const state = get();

    // 如果已经在缓存中，检查是否需要更新访问时间（超过1秒才更新，避免频繁更新）
    if (state.cachedTabsMap.has(tabId)) {
      const cached = state.cachedTabsMap.get(tabId)!;
      const now = Date.now();
      const routeChanged = cached.path !== path || cached.pageId !== pageId;

      // 如果距离上次访问不到1秒且路由元数据未变化，不更新
      if (!routeChanged && now - cached.lastAccessTime < 1000) {
        return;
      }

      const newCachedTabsMap = new Map(state.cachedTabsMap);
      newCachedTabsMap.set(tabId, {
        ...cached,
        pageId,
        path,
        lastAccessTime: now,
      });
      const newCachedTabsList = Array.from(newCachedTabsMap.values());
      set({ cachedTabsMap: newCachedTabsMap, cachedTabsList: newCachedTabsList });
      return;
    }

    // 检查缓存数量是否超过限制
    const newCachedTabsMap = new Map(state.cachedTabsMap);

    if (newCachedTabsMap.size >= state.maxCachedTabs) {
      // 找到最久未访问的 tab 并移除
      let oldestTabId: string | null = null;
      let oldestTime = Date.now();

      newCachedTabsMap.forEach((info, id) => {
        if (info.lastAccessTime < oldestTime) {
          oldestTime = info.lastAccessTime;
          oldestTabId = id;
        }
      });

      if (oldestTabId) {
        newCachedTabsMap.delete(oldestTabId);
      }
    }

    // 添加新的缓存
    newCachedTabsMap.set(tabId, {
      tabId,
      path,
      pageId,
      lastAccessTime: Date.now(),
      revision: 0,
    });

    const newCachedTabsList = Array.from(newCachedTabsMap.values());
    set({ cachedTabsMap: newCachedTabsMap, cachedTabsList: newCachedTabsList });
  },

  removeFromCache: (tabId) => {
    set((state) => {
      const newCachedTabsMap = new Map(state.cachedTabsMap);
      newCachedTabsMap.delete(tabId);
      const newCachedTabsList = Array.from(newCachedTabsMap.values());
      return { cachedTabsMap: newCachedTabsMap, cachedTabsList: newCachedTabsList };
    });
  },

  isTabCached: (tabId) => {
    return get().cachedTabsMap.has(tabId);
  },
}));
