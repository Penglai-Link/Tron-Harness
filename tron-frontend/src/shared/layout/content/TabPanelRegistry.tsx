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
  rightPanel?: ReactNode;
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
}

interface TabPanelRegistryValue {
  cacheConfigs: Readonly<Record<string, TabPanelConfig>>;
  clear: (tabId: string) => void;
  get: (tabId: string) => TabPanelConfig | undefined;
  register: (tabId: string, config: TabPanelConfig) => void;
  registerRightPanelTabHandler: (
    tabId: string,
    handler: (tab: EditableTab) => void,
  ) => () => void;
  openRightPanelTab: (tabId: string, tab: EditableTab) => void;
}

const emptyRegistry: TabPanelRegistryValue = {
  cacheConfigs: {},
  clear: () => undefined,
  get: () => undefined,
  register: () => undefined,
  registerRightPanelTabHandler: () => () => undefined,
  openRightPanelTab: () => undefined,
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
  const rightPanelTabHandlersRef = useRef(
    new Map<string, (tab: EditableTab) => void>(),
  );

  const registerRightPanelTabHandler = useCallback(
    (tabId: string, handler: (tab: EditableTab) => void) => {
      rightPanelTabHandlersRef.current.set(tabId, handler);
      return () => {
        if (rightPanelTabHandlersRef.current.get(tabId) === handler) {
          rightPanelTabHandlersRef.current.delete(tabId);
        }
      };
    },
    [],
  );

  const openRightPanelTab = useCallback((tabId: string, tab: EditableTab) => {
    rightPanelTabHandlersRef.current.get(tabId)?.(tab);
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
      return next;
    });
  }, [cachedTabsList]);

  const get = useCallback(
    (tabId: string) => cacheConfigs[tabId],
    [cacheConfigs],
  );

  const value = useMemo<TabPanelRegistryValue>(
    () => ({
      cacheConfigs,
      clear,
      get,
      openRightPanelTab,
      register,
      registerRightPanelTabHandler,
    }),
    [
      cacheConfigs,
      clear,
      get,
      openRightPanelTab,
      register,
      registerRightPanelTabHandler,
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
    config.rightPanel,
    config.searchPlaceholder,
    config.topbarActions,
    register,
    scopeTabId,
  ]);
}

export function useRightPanelTabs() {
  const registry = useTabPanelRegistry();
  const scopeTabId = useContext(TabPanelScopeContext);
  const updateRightPanelState = useTabStore(
    (state) => state.updateRightPanelState,
  );

  const openTab = useCallback(
    (tab: EditableTab) => {
      if (scopeTabId) {
        registry.openRightPanelTab(scopeTabId, tab);
        updateRightPanelState(scopeTabId, { open: true });
      }
    },
    [registry, scopeTabId, updateRightPanelState],
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
