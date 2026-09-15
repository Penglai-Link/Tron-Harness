import { lazy, memo, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  EditableTabs,
  TabVisibilityProvider,
  type EditableTab,
  type EditableTabEditInfo,
} from "@/components/editable-tabs";
import { OperateType } from "@/components/editable-tabs/types";
import { FileTabCoordinatorProvider } from "@/components/files/FileTabCoordinator";
import { PageView } from "@/components/page-view";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { DEFAULT_TAB_ID, useTabStore, type CachedTabInfo } from "@/store/tabStore";
import { isDesktopHost } from "@/shared/desktop/DesktopBridge";
import { FolderOpen, Globe, Plus } from "lucide-react";
import { useTabPanelRegistry } from "./TabPanelRegistry";

const FilePanel = lazy(() =>
  import("@/components/files/FilePanel").then((module) => ({ default: module.FilePanel })),
);

interface RightPanelTabsProps {
  tab: CachedTabInfo;
}

interface EmptyPanelContentProps {
  onAddFile?: () => void;
  onAddWeb?: () => void;
}

function EmptyPanelContent({ onAddFile, onAddWeb }: EmptyPanelContentProps) {
  const { t } = useTranslation("common");

  if (!onAddFile && !onAddWeb) {
    return (
      <div
        className="flex h-full min-h-32 flex-col items-center justify-center gap-3 px-6 text-sm text-[var(--color-on-surface-variant)]"
        data-testid="tron-right-sidebar-empty"
      >
        {t("content.noData")}
      </div>
    );
  }

  return (
    <div
      className="flex h-full min-h-32 flex-col items-center justify-center gap-3 px-6 text-sm text-[var(--color-on-surface-variant)]"
      data-testid="tron-right-sidebar-empty"
    >
      <div className="flex w-full flex-col gap-2">
        {onAddFile ? (
          <Button
            aria-label={t("content.newFileTab")}
            onClick={onAddFile}
            type="button"
            size="lg"
            variant="outline"
            className="w-full flex items-center justify-start"
          >
            <FolderOpen aria-hidden="true" />
            {t("files.panelTitle")}
          </Button>
        ) : null}
        {onAddWeb ? (
          <Button
            aria-label={t("content.newTab")}
            onClick={onAddWeb}
            type="button"
            size="lg"
            variant="outline"
            className="w-full flex items-center justify-start"
          >
            <Globe aria-hidden="true" />
            {t("content.browser")}
          </Button>
        ) : null}
      </div>
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
  const filesTabKeyPrefix = `page-files:${tab.tabId}:`;
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

  const [fileTabIds, setFileTabIds] = useState<string[]>([]);
  const nextFileTabId = useRef(1);

  const filesTabs = useMemo<EditableTab[]>(() => {
    if (!isDesktop) return [];

    return fileTabIds.map((key) => ({
      key,
      label: t("files.panelTitle"),
      closable: true,
      children: (
        <Suspense
          fallback={
            <div className="flex h-full items-center justify-center text-sm text-[var(--color-on-surface-variant)]">
              {t("files.loading")}
            </div>
          }
        >
          <FilePanel tabKey={key} />
        </Suspense>
      ),
    }));
  }, [fileTabIds, isDesktop, t]);

  const createWebTab = useCallback((): EditableTab => {
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

  const openFileTab = useCallback(() => {
    const id = nextFileTabId.current++;
    const key = `${filesTabKeyPrefix}${id}`;
    setFileTabIds((previous) => [...previous, key]);
    setRightPanelActiveKey(tab.tabId, key);
  }, [filesTabKeyPrefix, setRightPanelActiveKey, tab.tabId]);

  const focusFileTab = useCallback((key: string) => {
    setRightPanelActiveKey(tab.tabId, key);
  }, [setRightPanelActiveKey, tab.tabId]);

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
    () => [...filesTabs, pageAttributesTab, ...dynamicItems].filter((item) => !!item),
    [filesTabs, dynamicItems, pageAttributesTab],
  );

  const defaultActiveKey =
    pageAttributesTab?.key ?? filesTabs[0]?.key ?? items[0]?.key ?? "";
  const activeKey = getRightPanelActiveKey(tab.tabId) ?? defaultActiveKey;

  const handleEdit = useCallback(
    (action: OperateType, info: EditableTabEditInfo) => {
      if (action !== OperateType.Remove || !info.key) return;

      if (info.key.startsWith(filesTabKeyPrefix)) {
        setFileTabIds((previous) => previous.filter((key) => key !== info.key));
        if (info.remainingTabs?.length === 0) {
          updateRightPanelState(tab.tabId, { open: false });
        }
        if (activeKey === info.key) {
          const fallbackKey = info.remainingTabs?.find(
            (item) => item.key !== info.key,
          )?.key;
          setRightPanelActiveKey(tab.tabId, fallbackKey ?? defaultActiveKey);
        }
        return;
      }

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
    [activeKey, defaultActiveKey, filesTabKeyPrefix, removeRightPanelTab, setRightPanelActiveKey, tab.tabId, updateRightPanelState],
  );

  if (!items.length) {
    return (
      <EmptyPanelContent
        onAddFile={isDesktop ? openFileTab : undefined}
        onAddWeb={createWebTab}
      />
    );
  }

  return (
    <FileTabCoordinatorProvider onFocusTab={focusFileTab}>
      <EditableTabs
        className="h-full min-h-0"
        activeKey={activeKey}
        createTab={createWebTab}
        items={items}
        keepMounted
        preserveInternalTabs={false}
        provideTabVisibility
        onEdit={handleEdit}
        onChange={(key) => {
          if (key) setRightPanelActiveKey(tab.tabId, key);
        }}
        renderAddTrigger={() => (
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  aria-label={t("content.addTab")}
                  size="icon-sm"
                  type="button"
                  variant="ghost"
                >
                  <Plus aria-hidden="true" />
                </Button>
              }
            />
            <DropdownMenuContent align="end" side="bottom" sideOffset={4} className="w-44">
              {isDesktop ? (
                <DropdownMenuItem onClick={openFileTab}>
                  <FolderOpen size={14} aria-hidden="true" />
                  <span>{t("content.newFileTab")}</span>
                </DropdownMenuItem>
              ) : null}
              <DropdownMenuItem onClick={createWebTab}>
                <Globe size={14} aria-hidden="true" />
                <span>{t("content.newWebTab")}</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      />
    </FileTabCoordinatorProvider>
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
