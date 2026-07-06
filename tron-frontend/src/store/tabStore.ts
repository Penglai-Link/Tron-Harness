import { create } from "zustand";

export interface TabItem {
  id: string;
  title: string;
  path: string;
  closable: boolean;
  activeNav?: string;
}

// 缓存的 tab 信息
export interface CachedTabInfo {
  tabId: string;
  path: string;
  pageId: string;
  lastAccessTime: number;
}

interface TabState {
  tabs: TabItem[];
  activeTabId: string | null;
  cachedTabsMap: Map<string, CachedTabInfo>; // key: tabId, value: 缓存信息
  cachedTabsList: CachedTabInfo[]; // 缓存的 tab 列表，用于渲染
  maxCachedTabs: number; // 最大缓存数量
  addTab: (tab: Omit<TabItem, "id">) => string;
  removeTab: (tabId: string) => void;
  setActiveTab: (tabId: string) => void;
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

    // 如果有 activeNav，优先根据 activeNav 查找已存在的 tab
    if (tab.activeNav) {
      const existingTabByNav = state.tabs.find(t => t.activeNav === tab.activeNav);
      if (existingTabByNav) {
        // 检查 path 是否改变
        const pathChanged = existingTabByNav.path !== tab.path;

        // 检查是否需要更新
        const needsUpdate =
          pathChanged ||
          existingTabByNav.title !== tab.title ||
          state.activeTabId !== existingTabByNav.id;

        if (needsUpdate) {
          // 如果 path 改变了，需要清除该 tab 的缓存，让它重新渲染
          if (pathChanged) {
            const newCachedTabsMap = new Map(state.cachedTabsMap);
            newCachedTabsMap.delete(existingTabByNav.id);
            const newCachedTabsList = Array.from(newCachedTabsMap.values());

            set({
              tabs: state.tabs.map(t =>
                t.id === existingTabByNav.id
                  ? { ...t, path: tab.path, title: tab.title }
                  : t
              ),
              activeTabId: existingTabByNav.id,
              cachedTabsMap: newCachedTabsMap,
              cachedTabsList: newCachedTabsList,
            });
          } else {
            set({
              tabs: state.tabs.map(t =>
                t.id === existingTabByNav.id
                  ? { ...t, path: tab.path, title: tab.title }
                  : t
              ),
              activeTabId: existingTabByNav.id,
            });
          }
        }
        return existingTabByNav.id;
      }
    }

    // 否则按原逻辑根据 path 查找
    const existingTab = state.tabs.find(t => t.path === tab.path);

    if (existingTab) {
      // 只有当 activeTabId 不同时才更新
      if (state.activeTabId !== existingTab.id) {
        set({ activeTabId: existingTab.id });
      }
      return existingTab.id;
    }

    const newTab: TabItem = {
      ...tab,
      id: generateTabId(),
    };

    set({
      tabs: [...state.tabs, newTab],
      activeTabId: newTab.id,
    });

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

      // 从缓存中移除
      const newCachedTabsMap = new Map(state.cachedTabsMap);
      newCachedTabsMap.delete(tabId);
      const newCachedTabsList = Array.from(newCachedTabsMap.values());

      return {
        tabs,
        activeTabId: nextActiveTabId,
        cachedTabsMap: newCachedTabsMap,
        cachedTabsList: newCachedTabsList,
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
    set({ tabs: [], activeTabId: null, cachedTabsMap: new Map(), cachedTabsList: [] });
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

      // 如果距离上次访问不到1秒，不更新
      if (now - cached.lastAccessTime < 1000) {
        return;
      }

      const newCachedTabsMap = new Map(state.cachedTabsMap);
      newCachedTabsMap.set(tabId, {
        ...cached,
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
