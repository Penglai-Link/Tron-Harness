import {
  Activity,
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import {
  EditableTabs,
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
  const { get, registerRightPanelTabHandler } = useTabPanelRegistry();
  const updateRightPanelState = useTabStore(
    (state) => state.updateRightPanelState,
  );
  const nextTabId = useRef(1);
  const panelConfig = get(tab.tabId);
  const [dynamicTabs, setDynamicTabs] = useState<EditableTab[]>([]);
  const [activeKey, setActiveKey] = useState(`page-attributes:${tab.tabId}`);
  const pageAttributesTab = useMemo<EditableTab | null>(() => {
    return panelConfig?.rightPanel
      ? {
          key: `page-attributes:${tab.tabId}`,
          label: t('content.attributes'),
          closable: false,
          children: panelConfig?.rightPanel,
        }
      : null;
  }, [panelConfig?.rightPanel, tab.tabId]);

  const addRightPanelTab = useCallback(
    (newTab: EditableTab) => {
      if (newTab.key === `page-attributes:${tab.tabId}`) {
        return;
      }

      setActiveKey(newTab.key);

      setDynamicTabs((currentTabs) => {
        const existingIndex = currentTabs.findIndex(
          (item) => item.key === newTab.key,
        );
        if (existingIndex < 0) return [...currentTabs, newTab];
        const nextTabs = [...currentTabs];
        nextTabs[existingIndex] = newTab;
        return nextTabs;
      });
    },
    [tab.tabId],
  );

  useEffect(
    () => registerRightPanelTabHandler(tab.tabId, addRightPanelTab),
    [addRightPanelTab, registerRightPanelTabHandler, tab.tabId],
  );

  const createTab = useCallback((): EditableTab => {
    const id = nextTabId.current++;
    const key = `right-panel-page-${tab.tabId}-${id}`;
    const newTab: EditableTab = {
      key,
      label: t("content.newTab"),
      closable: true,
      children: <PageView url="" />,
    };
    addRightPanelTab(newTab);
    setActiveKey(newTab.key);
    return newTab;
  }, [addRightPanelTab, t, tab.tabId]);

  const handleEdit = useCallback(
    (action: OperateType, info: EditableTabEditInfo) => {
      if (action !== OperateType.Remove || !info.key) return;
      setDynamicTabs((currentTabs) =>
        currentTabs.filter((item) => item.key !== info.key),
      );
      if (info.remainingTabs?.length === 0) {
        updateRightPanelState(tab.tabId, { open: false });
      }
      if (activeKey === info.key) {
        const fallbackKey = info.remainingTabs?.find(
          (item) => item.key !== info.key,
        )?.key;
        setActiveKey(fallbackKey ?? `page-attributes:${tab.tabId}`);
      }
    },
    [activeKey, tab.tabId, updateRightPanelState],
  );

  const items = useMemo(
    () => [pageAttributesTab, ...dynamicTabs].filter((item) => !!item),
    [dynamicTabs, pageAttributesTab],
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
      onEdit={handleEdit}
      onChange={(key) => {
        if (key) setActiveKey(key);
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
        <Activity
          key={tab.tabId}
          mode={tab.tabId === activeTabId ? "visible" : "hidden"}
        >
          <RightPanelTabs tab={tab} />
        </Activity>
      ))}
    </>
  );
});

export default RightPanelContainer;
