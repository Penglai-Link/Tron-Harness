import { Plus } from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { DEFAULT_TAB_ID, panelTabId, useTabStore, type CachedTabInfo } from "@/store/tabStore";
import { FILE_TAB_KEY_PREFIX, useFileTabsStore } from "@/store/fileTabsStore";
import { isDesktopHost } from "@/shared/desktop/DesktopBridge";

import { useFilePanelTarget } from "./FilePanelHostProvider";
import { useTabPanelRegistry } from "./TabPanelRegistry";
import { WorkspaceTabScope } from "./WorkspaceTabScope";

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
    detachRightPanelTab, detachedRightPanelIds, returnRightPanelTab, focusRightPanelWindow,
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

  const panelId = panelTabId(tab.tabId, tab.workspaceCode);
  const panelConfig = get(panelId);
  const dynamicTabs = getRightPanelTabs(panelId);
  const ownerKey = getRightPanelOwnerKey(panelId);
  const pageAttributesKey = `page-attributes:${panelId}`;
  const isDesktop = isDesktopHost();
  const pageAttributesTab = useMemo<EditableTab | null>(() => {
    return panelConfig?.rightPanel
      ? {
          key: pageAttributesKey,
          label: panelConfig.rightPanelLabel ?? t('content.attributes'),
          closable: false,
          children: panelConfig?.rightPanel,
        }
      : null;
  }, [pageAttributesKey, panelConfig?.rightPanel, panelConfig?.rightPanelLabel, panelId]);
  const configuredTabs = panelConfig?.rightPanelTabs ?? [];

  const fileTabKeys = useFileTabsStore((state) => state.fileTabKeys);
  const fileTabLabels = useFileTabsStore((state) => state.fileTabLabels);
  const fileTabOwners = useFileTabsStore((state) => state.fileTabOwners);
  const activeFileTabKey = useFileTabsStore((state) => state.activeKeysByOwner[panelId] ?? null);
  const openFileTab = useFileTabsStore((state) => state.openFileTab);
  const closeFileTab = useFileTabsStore((state) => state.closeFileTab);
  const setActiveFileTab = useFileTabsStore((state) => state.setActiveFileTab);

  const filesTabs = useMemo<EditableTab[]>(() => {
    if (!isDesktop) return [];

    return fileTabKeys.filter((key) => fileTabOwners[key] === panelId).map((key) => ({
      key,
      label: fileTabLabels[key] || t("files.panelTitle"),
      labelClassName: "max-w-[100px]",
      closable: true,
      // 实际内容由全局 FilePanelHost 渲染，这里仅作为 tab 标签占位。
      children: <div className="h-full w-full" />,
    }));
  }, [fileTabKeys, fileTabLabels, fileTabOwners, panelId, isDesktop, t]);

  const createWebTab = useCallback((): EditableTab => {
    const id = crypto.randomUUID();
    const stableId = `right-panel-page-${panelId}-${ownerKey ?? "default"}-${id}`;
    const newTab: EditableTab = {
      key: stableId,
      stableId,
      url: "",
      label: t("content.newTab"),
      ownerKey,
      closable: true,
      children: null,
    };
    setActiveFileTab(null, panelId);
    openRightPanelTab(panelId, newTab);
    return newTab;
  }, [openRightPanelTab, ownerKey, setActiveFileTab, t, panelId]);

  const handleOpenFileTab = useCallback(() => {
    openFileTab(panelId);
    updateRightPanelState(tab.tabId, { open: true });
  }, [openFileTab, panelId, tab.tabId, updateRightPanelState]);

  const dynamicItems = useMemo(
    () => dynamicTabs.map((item) => {
      if (item.url === undefined || !item.stableId) return item;

      return {
        ...item,
        hidden: item.ownerKey !== ownerKey,
        detachable: !detachedRightPanelIds.includes(item.stableId),
        children: detachedRightPanelIds.includes(item.stableId) ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-sm">
            <p className="text-muted-foreground">{t("content.tabDetached")}</p>
            <div className="flex flex-wrap justify-center gap-2">
              <Button variant="outline" onClick={() => focusRightPanelWindow(item.stableId!)}>{t("content.focusWindow")}</Button>
              <Button onClick={() => returnRightPanelTab(item.stableId!)}>{t("content.returnTab")}</Button>
            </div>
          </div>
        ) : (
          <PageView
            inspectionMode={Boolean(panelConfig?.onPageInspectionConfirm)}
            onInspectionConfirm={panelConfig?.onPageInspectionConfirm}
            pageId={item.stableId}
            url={item.url}
            onUrlChange={(url) => {
              updateRightPanelTab(panelId, item.stableId!, {
                key: `${item.stableId}:${url}`,
                label: url,
                url,
              });
            }}
          />
        ),
      };
    }),
    [dynamicTabs, ownerKey, panelConfig?.onPageInspectionConfirm, panelId, updateRightPanelTab, detachedRightPanelIds, focusRightPanelWindow, returnRightPanelTab, t],
  );

  const items = useMemo(
    () => [...configuredTabs, pageAttributesTab, ...dynamicItems, ...filesTabs].filter(
      (item): item is EditableTab => item !== null,
    ),
    [configuredTabs, filesTabs, dynamicItems, pageAttributesTab],
  );

  const defaultActiveKey =
    configuredTabs[0]?.key ?? pageAttributesTab?.key ?? dynamicItems[0]?.key ?? filesTabs[0]?.key ?? "";
  const routeActiveKey = getRightPanelActiveKey(panelId);
  const requestedActiveKey = activeFileTabKey ?? routeActiveKey ?? defaultActiveKey;
  const activeKey = items.some((item) => item.key === requestedActiveKey && !item.hidden)
    ? requestedActiveKey
    : defaultActiveKey;

  useEffect(() => {
    if (!activeFileTabKey && routeActiveKey && routeActiveKey !== activeKey) {
      setRightPanelActiveKey(panelId, activeKey);
    }
  }, [activeFileTabKey, activeKey, routeActiveKey, setRightPanelActiveKey, panelId]);

  const handleEdit = useCallback(
    (action: OperateType, info: EditableTabEditInfo) => {
      if (action !== OperateType.Remove || !info.key) return;

      if (info.key.startsWith(FILE_TAB_KEY_PREFIX)) {
        closeFileTab(info.key);
        if (info.remainingTabs?.length === 0) {
          updateRightPanelState(tab.tabId, { open: false });
        }
        return;
      }

      removeRightPanelTab(panelId, info.key);
      if (info.remainingTabs?.length === 0) {
        updateRightPanelState(tab.tabId, { open: false });
      }
      if (activeKey === info.key) {
        const fallbackKey = info.remainingTabs?.find(
          (item) => item.key !== info.key,
        )?.key;
        setRightPanelActiveKey(panelId, fallbackKey ?? defaultActiveKey);
      }
    },
    [activeKey, closeFileTab, defaultActiveKey, removeRightPanelTab, setRightPanelActiveKey, panelId, tab.tabId, updateRightPanelState],
  );

  if (!items.length) {
    return (
      <EmptyPanelContent
        onAddFile={isDesktop ? handleOpenFileTab : undefined}
        onAddWeb={createWebTab}
      />
    );
  }

  return (
    <EditableTabs
      className="h-full min-h-0"
      activeKey={activeKey}
      createTab={createWebTab}
      items={items}
      keepMounted
      preserveInternalTabs={false}
      provideTabVisibility
      onEdit={handleEdit}
      onDetach={(item, position) => detachRightPanelTab(panelId, item, position)}
      onChange={(key) => {
        if (!key) return;
        if (key.startsWith(FILE_TAB_KEY_PREFIX)) {
          setActiveFileTab(key, panelId);
        } else {
          setActiveFileTab(null, panelId);
          setRightPanelActiveKey(panelId, key);
        }
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
            {isDesktop && !panelConfig?.rightPanelBrowserOnly ? (
              <DropdownMenuItem onClick={handleOpenFileTab}>

                <span>{t("content.newFileTab")}</span>
              </DropdownMenuItem>
            ) : null}
            <DropdownMenuItem onClick={createWebTab}>

              <span>{t("content.newWebTab")}</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    />
  );
});

const RightPanelContainer = memo(function RightPanelContainer() {
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);
  const cachedTabs = useTabStore((state) => state.cachedTabsList);
  const workspaceCode = useTabStore((state) => state.workspaceCode);
  const workspaceTabs = useTabStore((state) => state.workspaceTabs);
  const previousActiveTabId = useRef(activeTabId);
  const mountFilePanel = useFilePanelTarget();
  const hostRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (hostRef.current) mountFilePanel(hostRef.current);
  }, [mountFilePanel]);
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
    const spaces = new Set([workspaceCode, ...Object.keys(workspaceTabs)]);
    const defaultPanels: CachedTabInfo[] = Array.from(spaces).filter((code) => code !== null).map((code) => ({
      tabId: DEFAULT_TAB_ID,
      path: "/default",
      pageId: "",
      lastAccessTime: 0,
      revision: 0,
      workspaceCode: code ?? undefined,
    }));
    return [...cachedTabs, ...defaultPanels];
  }, [cachedTabs, workspaceCode, workspaceTabs]);

  if (!panelTabs.length) {
    return <EmptyPanelContent />;
  }

  if (switchingToDefault) {
    return <div className="h-full w-full" data-testid="tron-right-sidebar-transition" />;
  }

  return (
    <div ref={hostRef} className="relative h-full w-full overflow-hidden">
      {panelTabs.map((tab) => {
        const isActive = tab.tabId === activeTabId && tab.workspaceCode === workspaceCode;
        return (
          <div
            key={panelTabId(tab.tabId, tab.workspaceCode)}
            aria-hidden={!isActive}
            className={
              isActive
                ? "relative z-10 h-full w-full"
                : "absolute inset-0 z-0 h-full w-full invisible pointer-events-none"
            }
          >
            <TabVisibilityProvider visible={isActive}>
              <WorkspaceTabScope workspaceCode={tab.workspaceCode}>
                <RightPanelTabs tab={tab} />
              </WorkspaceTabScope>
            </TabVisibilityProvider>
          </div>
        );
      })}
    </div>
  );
});

export default RightPanelContainer;
