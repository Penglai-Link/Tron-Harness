import {
  Activity,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { useTabStore } from "@/store/tabStore";
import type { EditableTab } from "@/components/editable-tabs";

export interface TabPanelConfig {
  contentClassName?: string;
  onPageInspectionConfirm?: (result: import("@/components/page-view").TargetPageFrameInspectionResult) => void;
  rightPanel?: ReactNode;
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
}

interface TabPanelRegistryValue {
  cacheConfigs: Readonly<Record<string, TabPanelConfig>>;
  clear: (tabId: string) => void;
  get: (tabId: string) => TabPanelConfig | undefined;
  getRightPanelActiveKey: (tabId: string) => string | undefined;
  getRightPanelOwnerKey: (tabId: string) => string | undefined;
  getRightPanelTabs: (tabId: string) => readonly EditableTab[];
  register: (tabId: string, config: TabPanelConfig) => void;
  openRightPanelTab: (tabId: string, tab: EditableTab) => void;
  updateRightPanelTab: (
    tabId: string,
    stableId: string,
    updates: Partial<EditableTab>,
  ) => void;
  removeRightPanelTab: (tabId: string, key: string) => void;
  setRightPanelActiveKey: (tabId: string, key: string) => void;
  setRightPanelOwnerKey: (tabId: string, ownerKey: string | undefined) => void;
}

const emptyRegistry: TabPanelRegistryValue = {
  cacheConfigs: {},
  clear: () => undefined,
  get: () => undefined,
  getRightPanelActiveKey: () => undefined,
  getRightPanelOwnerKey: () => undefined,
  getRightPanelTabs: () => [],
  register: () => undefined,
  openRightPanelTab: () => undefined,
  updateRightPanelTab: () => undefined,
  removeRightPanelTab: () => undefined,
  setRightPanelActiveKey: () => undefined,
  setRightPanelOwnerKey: () => undefined,
};

const TabPanelRegistryContext =
  createContext<TabPanelRegistryValue>(emptyRegistry);
const TabPanelScopeContext = createContext<string | null>(null);

interface TabPanelRegistryProviderProps {
  tabId?: string;
  children: ReactNode;
}

export function TabPanelRegistryProvider({
  tabId,
  children,
}: TabPanelRegistryProviderProps) {
  const parentRegistry = useContext(TabPanelRegistryContext);

  if (tabId !== undefined) {
    return (
      <TabPanelRegistryScope parentRegistry={parentRegistry} tabId={tabId}>
        {children}
      </TabPanelRegistryScope>
    );
  }

  return <TabPanelRegistryRoot>{children}</TabPanelRegistryRoot>;
}

function TabPanelRegistryRoot({ children }: { children: ReactNode }) {
  const cachedTabsList = useTabStore((state) => state.cachedTabsList);
  const [cacheConfigs, setCacheConfigs] = useState<
    Record<string, TabPanelConfig>
  >({});
  const [rightPanelTabs, setRightPanelTabs] = useState<
    Record<string, readonly EditableTab[]>
  >({});
  const [rightPanelActiveKeys, setRightPanelActiveKeys] = useState<
    Record<string, string>
  >({});
  const [rightPanelOwnerKeys, setRightPanelOwnerKeys] = useState<
    Record<string, string | undefined>
  >({});

  const activeKeyScope = useCallback(
    (tabId: string, ownerKey?: string) => `${tabId}\u0000${ownerKey ?? ""}`,
    [],
  );

  const openRightPanelTab = useCallback((tabId: string, tab: EditableTab) => {
    setRightPanelTabs((current) => {
      const currentTabs = current[tabId] ?? [];
      if (currentTabs.some((item) => item.key === tab.key)) return current;
      return { ...current, [tabId]: [...currentTabs, tab] };
    });
    const scope = activeKeyScope(tabId, tab.ownerKey);
    setRightPanelActiveKeys((current) => (
      current[scope] === tab.key ? current : { ...current, [scope]: tab.key }
    ));
  }, [activeKeyScope]);

  const updateRightPanelTab = useCallback(
    (tabId: string, stableId: string, updates: Partial<EditableTab>) => {
      setRightPanelTabs((current) => {
        const currentTabs = current[tabId] ?? [];
        const previous = currentTabs.find((tab) => tab.stableId === stableId);
        if (!previous) return current;

        const nextTabs = currentTabs.map((tab) =>
          tab.stableId === stableId ? { ...tab, ...updates } : tab,
        );
        return { ...current, [tabId]: nextTabs };
      });
      setRightPanelActiveKeys((current) => {
        const previous = (rightPanelTabs[tabId] ?? []).find(
          (tab) => tab.stableId === stableId,
        );
        const scope = activeKeyScope(tabId, previous?.ownerKey);
        if (!previous || current[scope] !== previous.key || !updates.key) {
          return current;
        }
        return { ...current, [scope]: updates.key };
      });
    },
    [activeKeyScope, rightPanelTabs],
  );

  const removeRightPanelTab = useCallback((tabId: string, key: string) => {
    setRightPanelTabs((current) => {
      const currentTabs = current[tabId] ?? [];
      const nextTabs = currentTabs.filter((tab) => tab.key !== key);
      if (nextTabs.length === currentTabs.length) return current;
      return { ...current, [tabId]: nextTabs };
    });
  }, []);

  const setRightPanelActiveKey = useCallback((tabId: string, key: string) => {
    const ownerKey = rightPanelTabs[tabId]?.find((tab) => tab.key === key)?.ownerKey
      ?? rightPanelOwnerKeys[tabId];
    const scope = activeKeyScope(tabId, ownerKey);
    setRightPanelActiveKeys((current) => (
      current[scope] === key ? current : { ...current, [scope]: key }
    ));
  }, [activeKeyScope, rightPanelOwnerKeys, rightPanelTabs]);

  const setRightPanelOwnerKey = useCallback((tabId: string, ownerKey: string | undefined) => {
    setRightPanelOwnerKeys((current) => (
      current[tabId] === ownerKey ? current : { ...current, [tabId]: ownerKey }
    ));
  }, []);

  const register = useCallback((tabId: string, config: TabPanelConfig) => {
    const definedConfigEntries = Object.entries(config).filter(
      ([, value]) => value !== undefined,
    );
    if (!definedConfigEntries.length) return;

    setCacheConfigs((current) => {
      const previous = current[tabId];
      const nextConfig = {
        ...previous,
        ...Object.fromEntries(definedConfigEntries),
      };
      if (
        previous &&
        previous.contentClassName === nextConfig.contentClassName &&
        previous.onPageInspectionConfirm === nextConfig.onPageInspectionConfirm &&
        previous.rightPanel === nextConfig.rightPanel &&
        previous.searchPlaceholder === nextConfig.searchPlaceholder &&
        previous.topbarActions === nextConfig.topbarActions
      ) {
        return current;
      }
      return { ...current, [tabId]: nextConfig };
    });
  }, []);

  const clear = useCallback((tabId: string) => {
    setCacheConfigs((current) => {
      if (!Object.prototype.hasOwnProperty.call(current, tabId)) return current;

      const next = { ...current };
      delete next[tabId];
      return next;
    });
  }, []);

  useEffect(() => {
    const cachedTabIds = new Set(cachedTabsList.map((tab) => tab.tabId));
    setCacheConfigs((current) => {
      const staleTabIds = Object.keys(current).filter(
        (tabId) => !cachedTabIds.has(tabId),
      );
      if (!staleTabIds.length) return current;

      const next = { ...current };
      staleTabIds.forEach((tabId) => delete next[tabId]);
      setRightPanelTabs((currentTabs) => {
        const remaining = { ...currentTabs };
        staleTabIds.forEach((tabId) => delete remaining[tabId]);
        return remaining;
      });
      setRightPanelOwnerKeys((currentKeys) => {
        const remaining = { ...currentKeys };
        staleTabIds.forEach((tabId) => delete remaining[tabId]);
        return remaining;
      });
      setRightPanelActiveKeys((currentKeys) => {
        const remaining = { ...currentKeys };
        staleTabIds.forEach((tabId) => delete remaining[tabId]);
        return remaining;
      });
      return next;
    });
  }, [cachedTabsList]);

  const get = useCallback(
    (tabId: string) => cacheConfigs[tabId],
    [cacheConfigs],
  );
  const getRightPanelTabs = useCallback(
    (tabId: string) => rightPanelTabs[tabId] ?? [],
    [rightPanelTabs],
  );
  const getRightPanelActiveKey = useCallback(
    (tabId: string) => rightPanelActiveKeys[activeKeyScope(tabId, rightPanelOwnerKeys[tabId])],
    [activeKeyScope, rightPanelActiveKeys, rightPanelOwnerKeys],
  );
  const getRightPanelOwnerKey = useCallback(
    (tabId: string) => rightPanelOwnerKeys[tabId],
    [rightPanelOwnerKeys],
  );

  const value = useMemo<TabPanelRegistryValue>(
    () => ({
      cacheConfigs,
      clear,
      get,
      getRightPanelActiveKey,
      getRightPanelOwnerKey,
      getRightPanelTabs,
      openRightPanelTab,
      updateRightPanelTab,
      register,
      removeRightPanelTab,
      setRightPanelActiveKey,
      setRightPanelOwnerKey,
    }),
    [
      cacheConfigs,
      clear,
      get,
      getRightPanelActiveKey,
      getRightPanelOwnerKey,
      getRightPanelTabs,
      openRightPanelTab,
      updateRightPanelTab,
      register,
      removeRightPanelTab,
      setRightPanelActiveKey,
      setRightPanelOwnerKey,
    ],
  );

  return (
    <TabPanelRegistryContext.Provider value={value}>
      {children}
    </TabPanelRegistryContext.Provider>
  );
}

function TabPanelRegistryScope({
  children,
  parentRegistry,
  tabId,
}: {
  children: ReactNode;
  parentRegistry: TabPanelRegistryValue;
  tabId: string;
}) {
  return (
    <TabPanelRegistryContext.Provider value={parentRegistry}>
      <TabPanelScopeContext.Provider value={tabId}>
        {children}
      </TabPanelScopeContext.Provider>
    </TabPanelRegistryContext.Provider>
  );
}

export function useTabPanelRegistry() {
  return useContext(TabPanelRegistryContext);
}

export function useTabPanelConfig(config: TabPanelConfig) {
  const registry = useTabPanelRegistry();
  const scopeTabId = useContext(TabPanelScopeContext);
  const register = registry.register;

  useEffect(() => {
    if (scopeTabId) {
      register(scopeTabId, config);
    }
  }, [
    config.contentClassName,
    config.onPageInspectionConfirm,
    config.rightPanel,
    config.searchPlaceholder,
    config.topbarActions,
    register,
    scopeTabId,
  ]);
}

export function useRightPanelTabs(ownerKey?: string) {
  const registry = useTabPanelRegistry();
  const scopeTabId = useContext(TabPanelScopeContext);
  const updateRightPanelState = useTabStore(
    (state) => state.updateRightPanelState,
  );

  useEffect(() => {
    if (scopeTabId) {
      registry.setRightPanelOwnerKey(scopeTabId, ownerKey);
    }
  }, [ownerKey, registry, scopeTabId]);

  const openTab = useCallback(
    (tab: EditableTab) => {
      if (scopeTabId) {
        const scopedPrefix = ownerKey ? `${encodeURIComponent(ownerKey)}:` : "";
        registry.openRightPanelTab(scopeTabId, {
          ...tab,
          key: `${scopedPrefix}${tab.key}`,
          ownerKey,
          stableId: tab.stableId ? `${scopedPrefix}${tab.stableId}` : undefined,
        });
        updateRightPanelState(scopeTabId, { open: true });
      }
    },
    [ownerKey, registry, scopeTabId, updateRightPanelState],
  );

  return useMemo(() => ({ openTab }), [openTab]);
}

export function TabPanelHost() {
  const activeTabId = useTabStore((state) => state.activeTabId);
  const cachedTabs = useTabStore((state) => state.cachedTabsList);
  const { get } = useTabPanelRegistry();

  return cachedTabs.flatMap((cachedTab) => {
    const panel = get(cachedTab.tabId)?.rightPanel;
    if (!panel) return [];

    return (
      <Activity
        key={`right-panel:${cachedTab.tabId}:${cachedTab.revision}`}
        mode={cachedTab.tabId === activeTabId ? "visible" : "hidden"}
      >
        {panel}
      </Activity>
    );
  });
}
