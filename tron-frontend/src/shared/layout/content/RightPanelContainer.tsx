import { lazy, memo, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  EditableTabs,
  TabVisibilityProvider,
  type EditableTab,
  type EditableTabEditInfo,
} from "@/components/editable-tabs";
import { OperateType } from "@/components/editable-tabs/types";
import { PageView } from "@/components/page-view";
import { Button } from "@/components/ui/button";
import { DEFAULT_TAB_ID, useTabStore, type CachedTabInfo } from "@/store/tabStore";
import { isDesktopHost } from "@/shared/desktop/DesktopBridge";
import { Globe, Plus } from "lucide-react";
import { useTabPanelRegistry } from "./TabPanelRegistry";

const FilePanel = lazy(() =>
  import("@/components/files/FilePanel").then((module) => ({ default: module.FilePanel })),
);

interface RightPanelTabsProps {
  tab: CachedTabInfo;
}

interface EmptyPanelContentProps {
  onAdd?: () => void;
}

function EmptyPanelContent({ onAdd }: EmptyPanelContentProps) {
  const { t } = useTranslation("common");

  return (
    <div
      className="flex h-full min-h-32 flex-col items-center justify-center gap-3 px-6 text-sm text-[var(--color-on-surface-variant)]"
      data-testid="tron-right-sidebar-empty"
    >
      {onAdd ? (
        <Button
          aria-label={t("content.newTab")}
          onClick={onAdd}
          type="button"
          size="lg"
          variant="outline"
          className="w-full flex items-center justify-start"
        >
          <Globe />
          {t('content.browser')}
        </Button>
      ) : (
        t('content.noData')
      )}
    </div>
  );
}

const RightPanelTabs = memo(function RightPanelTabs({
  tab,
}: RightPanelTabsProps) {
  const { t } = useTranslation("common");
  const {
    get,
    getRightPanelActiveKey,
    getRightPanelOwnerKey,
    getRightPanelTabs,
    openRightPanelTab,
    removeRightPanelTab,
    setRightPanelActiveKey,
    updateRightPanelTab,
  } = useTabPanelRegistry();
  const updateRightPanelState = useTabStore(
    (state) => state.updateRightPanelState,
  );

  const nextTabId = useRef(1);
  const panelConfig = get(tab.tabId);
  const dynamicTabs = getRightPanelTabs(tab.tabId);
  const ownerKey = getRightPanelOwnerKey(tab.tabId);
  const pageAttributesKey = `page-attributes:${tab.tabId}`;
  const filesTabKey = `page-files:${tab.tabId}`;
  const isDesktop = isDesktopHost();
  const pageAttributesTab = useMemo<EditableTab | null>(() => {
    return panelConfig?.rightPanel
      ? {
          key: pageAttributesKey,
          label: t('content.attributes'),
          closable: false,
          children: panelConfig?.rightPanel,
        }
      : null;
  }, [pageAttributesKey, panelConfig?.rightPanel, tab.tabId]);

  const filesTab = useMemo<EditableTab | null>(() => {
    if (!isDesktop) return null;

    return {
      key: filesTabKey,
      label: t("files.panelTitle"),
      closable: false,
      children: (
        <Suspense
          fallback={
            <div className="flex h-full items-center justify-center text-sm text-[var(--color-on-surface-variant)]">
              {t("files.loading")}
            </div>
          }
        >
          <FilePanel />
        </Suspense>
      ),
    };
  }, [filesTabKey, isDesktop, t]);

  const createTab = useCallback((): EditableTab => {
    const id = nextTabId.current++;
    const stableId = `right-panel-page-${tab.tabId}-${ownerKey ?? "default"}-${id}`;
    const newTab: EditableTab = {
      key: stableId,
      stableId,
      url: "",
      label: t("content.newTab"),
      ownerKey,
      closable: true,
      children: null,
    };
    openRightPanelTab(tab.tabId, newTab);
    return newTab;
  }, [openRightPanelTab, ownerKey, t, tab.tabId]);

  const dynamicItems = useMemo(
    () => dynamicTabs.map((item) => {
      if (item.url === undefined || !item.stableId) return item;

      return {
        ...item,
        hidden: item.ownerKey !== ownerKey,
        children: (
          <PageView
            inspectionMode={Boolean(panelConfig?.onPageInspectionConfirm)}
            onInspectionConfirm={panelConfig?.onPageInspectionConfirm}
            pageId={item.stableId}
            url={item.url}
            onUrlChange={(url) => {
              updateRightPanelTab(tab.tabId, item.stableId!, {
                key: `${item.stableId}:${url}`,
                label: url,
                url,
              });
            }}
          />
        ),
      };
    }),
    [dynamicTabs, ownerKey, panelConfig?.onPageInspectionConfirm, tab.tabId, updateRightPanelTab],
  );

  const items = useMemo(
    () => [filesTab, pageAttributesTab, ...dynamicItems].filter((item) => !!item),
    [filesTab, dynamicItems, pageAttributesTab],
  );

  const defaultActiveKey =
    pageAttributesTab?.key ?? filesTab?.key ?? items[0]?.key ?? "";
  const activeKey = getRightPanelActiveKey(tab.tabId) ?? defaultActiveKey;

  const handleEdit = useCallback(
    (action: OperateType, info: EditableTabEditInfo) => {
      if (action !== OperateType.Remove || !info.key) return;
      removeRightPanelTab(tab.tabId, info.key);
      if (info.remainingTabs?.length === 0) {
        updateRightPanelState(tab.tabId, { open: false });
      }
      if (activeKey === info.key) {
        const fallbackKey = info.remainingTabs?.find(
          (item) => item.key !== info.key,
        )?.key;
        setRightPanelActiveKey(tab.tabId, fallbackKey ?? defaultActiveKey);
      }
    },
    [activeKey, defaultActiveKey, removeRightPanelTab, setRightPanelActiveKey, tab.tabId, updateRightPanelState],
  );

  if (!items.length) {
    return <EmptyPanelContent onAdd={createTab} />;
  }

  return (
    <EditableTabs
      className="h-full min-h-0"
      activeKey={activeKey}
      createTab={createTab}
      items={items}
      keepMounted
      preserveInternalTabs={false}
      provideTabVisibility
      onEdit={handleEdit}
      onChange={(key) => {
        if (key) setRightPanelActiveKey(tab.tabId, key);
      }}
    />
  );
});

const RightPanelContainer = memo(function RightPanelContainer() {
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);
  const cachedTabs = useTabStore((state) => state.cachedTabsList);
  const previousActiveTabId = useRef(activeTabId);
  const [settledActiveTabId, setSettledActiveTabId] = useState(activeTabId);
  const switchingToDefault =
    activeTabId === DEFAULT_TAB_ID &&
    previousActiveTabId.current !== DEFAULT_TAB_ID &&
    settledActiveTabId !== DEFAULT_TAB_ID;

  useEffect(() => {
    const wasOrdinaryTab = previousActiveTabId.current !== DEFAULT_TAB_ID;
    previousActiveTabId.current = activeTabId;

    if (activeTabId !== DEFAULT_TAB_ID || !wasOrdinaryTab) {
      setSettledActiveTabId(activeTabId);
      return;
    }

    const timer = window.setTimeout(
      () => setSettledActiveTabId(DEFAULT_TAB_ID),
      0,
    );
    return () => window.clearTimeout(timer);
  }, [activeTabId]);

  const panelTabs = useMemo(() => {
    const cachedTabIds = new Set(cachedTabs.map((tab) => tab.tabId));
    const uncachedPanels = !cachedTabIds.has(DEFAULT_TAB_ID)
      ? [{
        tabId: DEFAULT_TAB_ID,
        path: "/default",
        pageId: "",
        lastAccessTime: 0,
        revision: 0,
      }]
      : [];

    return [...cachedTabs, ...uncachedPanels];
  }, [cachedTabs]);

  if (!panelTabs.length) {
    return <EmptyPanelContent />;
  }

  if (switchingToDefault) {
    return <div className="h-full w-full" data-testid="tron-right-sidebar-transition" />;
  }

  return (
    <div className="relative h-full w-full overflow-hidden">
      {panelTabs.map((tab) => {
        const isActive = tab.tabId === activeTabId;
        return (
          <div
            key={tab.tabId}
            aria-hidden={!isActive}
            className={
              isActive
                ? "relative z-10 h-full w-full"
                : "absolute inset-0 z-0 h-full w-full invisible pointer-events-none"
            }
          >
            <TabVisibilityProvider visible={isActive}>
              <RightPanelTabs tab={tab} />
            </TabVisibilityProvider>
          </div>
        );
      })}
    </div>
  );
});

export default RightPanelContainer;
