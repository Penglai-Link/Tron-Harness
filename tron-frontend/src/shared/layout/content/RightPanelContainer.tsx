import { memo, useCallback, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

import { EditableTabs, type EditableTab } from "@/components/editable-tabs";
import { useTabStore } from "@/store/tabStore";
import { TabPanelHost, useTabPanelRegistry } from "./TabPanelRegistry";
import { PageView } from "@/shared/ui/page-view/Index";

const RightPanelContainer = memo(function RightPanelContainer() {
  const { t } = useTranslation("common");
  const nextTabId = useRef(1);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const { get } = useTabPanelRegistry();
  const hasPageAttributes = Boolean(
    activeTabId && get(activeTabId)?.rightPanel,
  );

  const pageAttributesTab = useMemo<EditableTab>(
    () => ({
      key: "page-attributes",
      label: "页面属性",
      closable: false,
      children: hasPageAttributes ? (
        <TabPanelHost />
      ) : (
        <div
          className="flex h-full min-h-32 items-center justify-center px-6 text-sm text-[var(--color-on-surface-variant)]"
          data-testid="tron-right-sidebar-empty"
        >
          {t("content.noData", "暂无属性")}
        </div>
      ),
    }),
    [hasPageAttributes, t],
  );




  const createTab = useCallback((url:string): EditableTab => {
    const tabId = nextTabId.current++;
    return {
      key: `right-panel-${tabId}`,
      label: url || '新标签页',
      closable: true,
      children: (
        <PageView
          url={url}
          onInspectionConfirm={(result) => {
            console.log('result',result)
          }}
        />
      ),
    };
  }, [t]);
  const items = useMemo(() => [pageAttributesTab], [pageAttributesTab]);

  return (
    <EditableTabs
      className="h-full min-h-0"
      createTab={() => createTab('')}
      items={items}
      keepMounted
    />
  );
});

export default RightPanelContainer;
