import { memo, useCallback, useMemo, useRef } from "react";
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
import { useTabStore, type CachedTabInfo } from "@/store/tabStore";
import { Globe, Plus } from "lucide-react";
import { useTabPanelRegistry } from "./TabPanelRegistry";

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
  const pageAttributesKey = `page-attributes:${tab.tabId}`;
  const activeKey = getRightPanelActiveKey(tab.tabId) ?? pageAttributesKey;
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

  const createTab = useCallback((): EditableTab => {
    const id = nextTabId.current++;
    const stableId = `right-panel-page-${tab.tabId}-${id}`;
    const newTab: EditableTab = {
      key: stableId,
      stableId,
      url: "",
      label: t("content.newTab"),
      closable: true,
      children: null,
    };
    openRightPanelTab(tab.tabId, newTab);
    return newTab;
  }, [openRightPanelTab, t, tab.tabId]);

  const dynamicItems = useMemo(
    () => dynamicTabs.map((item) => {
      if (item.url === undefined || !item.stableId) return item;

      return {
        ...item,
        children: (
          <PageView
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
    [dynamicTabs, tab.tabId, updateRightPanelTab],
  );

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
        setRightPanelActiveKey(tab.tabId, fallbackKey ?? pageAttributesKey);
      }
    },
    [activeKey, pageAttributesKey, removeRightPanelTab, setRightPanelActiveKey, tab.tabId, updateRightPanelState],
  );

  const items = useMemo(
    () => [pageAttributesTab, ...dynamicItems].filter((item) => !!item),
    [dynamicItems, pageAttributesTab],
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
      provideTabVisibility
      onEdit={handleEdit}
      onChange={(key) => {
        if (key) setRightPanelActiveKey(tab.tabId, key);
      }}
    />
  );
});

const RightPanelContainer = memo(function RightPanelContainer() {
  const activeTabId = useTabStore((state) => state.activeTabId);
  const cachedTabs = useTabStore((state) => state.cachedTabsList);

  if (!cachedTabs.length) {
    return <EmptyPanelContent />;
  }

  return (
    <>
      {cachedTabs.map((tab) => (
        <div
          key={tab.tabId}
          hidden={tab.tabId !== activeTabId}
          aria-hidden={tab.tabId !== activeTabId}
          className="h-full"
        >
          <TabVisibilityProvider visible={tab.tabId === activeTabId}>
            <RightPanelTabs tab={tab} />
          </TabVisibilityProvider>
        </div>
      ))}
    </>
  );
});

export default RightPanelContainer;
