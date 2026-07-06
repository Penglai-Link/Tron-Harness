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
