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
import { PageView } from "@/shared/ui/page-view/Index";
import { useTabStore, type CachedTabInfo } from "@/store/tabStore";
import { useTabPanelRegistry } from "./TabPanelRegistry";

interface RightPanelTabsProps {
  tab: CachedTabInfo;
}

function EmptyPanelContent() {
  const { t } = useTranslation("common");

  return (
    <div
      className="flex h-full min-h-32 items-center justify-center px-6 text-sm text-[var(--color-on-surface-variant)]"
      data-testid="tron-right-sidebar-empty"
    >
      {t("content.noData", "暂无属性")}
    </div>
  );
}

const RightPanelTabs = memo(function RightPanelTabs({
  tab,
}: RightPanelTabsProps) {
  const { t } = useTranslation("common");
  const { get, registerRightPanelTabHandler } = useTabPanelRegistry();
  const nextTabId = useRef(1);
  const panelConfig = get(tab.tabId);
  const [dynamicTabs, setDynamicTabs] = useState<EditableTab[]>([]);
  const pageAttributesTab = useMemo<EditableTab>(
    () => ({
      key: `page-attributes:${tab.tabId}`,
      label: "页面属性",
      closable: false,
      children: panelConfig?.rightPanel ?? <EmptyPanelContent />,
    }),
    [panelConfig?.rightPanel, tab.tabId],
  );

  const addRightPanelTab = useCallback(
    (newTab: EditableTab) => {
      if (newTab.key === `page-attributes:${tab.tabId}`) {
        return;
      }

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
      label: t("content.newTab", "新标签页"),
      closable: true,
      children: <PageView url="" />,
    };
    addRightPanelTab(newTab);
    return newTab;
  }, [addRightPanelTab, t, tab.tabId]);

  const handleEdit = useCallback(
    (action: OperateType, info: EditableTabEditInfo) => {
      if (action !== OperateType.Remove || !info.key) return;
      setDynamicTabs((currentTabs) =>
        currentTabs.filter((item) => item.key !== info.key),
      );
    },
    [],
  );

  const items = useMemo(
    () => [pageAttributesTab, ...dynamicTabs],
    [dynamicTabs, pageAttributesTab],
  );

  return (
    <EditableTabs
      className="h-full min-h-0"
      createTab={createTab}
      items={items}
      keepMounted
      onEdit={handleEdit}
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
