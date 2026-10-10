import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Tabs, TabsContent } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { MotionReveal } from "@/shared/motion";

import { EditableTabList } from "./EditableTabList";
import { TabVisibilityProvider } from "./TabVisibilityContext";
import { OperateType, type EditableTab, type EditableTabsProps } from "./types";

function getInitialActiveKey(
  items: readonly EditableTab[],
  defaultActiveKey?: string,
) {
  if (defaultActiveKey && items.some((item) => item.key === defaultActiveKey)) {
    return defaultActiveKey;
  }
  return items[0]?.key;
}

export function EditableTabs({
  items,
  closeTabRequest,
  activeKey,
  defaultActiveKey,
  createTab,
  hideAdd = false,
  renderAddTrigger,
  renderTrailingAction,
  onChange,
  onEdit,
  className,
  keepMounted,
  preserveInternalTabs = true,
  provideTabVisibility = false,
}: EditableTabsProps) {
  const processedCloseRequestId = useRef<number | undefined>(undefined);
  const [tabs, setTabs] = useState<EditableTab[]>(() => [...items]);
  const tabsRef = useRef(tabs);
  const [activeTabKey, setActiveTabKey] = useState<string | undefined>(() =>
    getInitialActiveKey(items, defaultActiveKey),
  );
  tabsRef.current = tabs;

  useEffect(() => {
    setTabs((currentTabs) => {
      const incomingKeys = new Set(items.map((item) => item.key));
      const incomingStableIds = new Set(
        items
          .map((item) => item.stableId)
          .filter((stableId): stableId is string => Boolean(stableId)),
      );
      const internalTabs = preserveInternalTabs
        ? currentTabs.filter(
            (item) =>
              !incomingKeys.has(item.key) &&
              !(item.stableId && incomingStableIds.has(item.stableId)),
          )
        : [];
      const nextTabs = items.map((item) => {
        const previous = currentTabs.find(
          (current) => current.stableId && current.stableId === item.stableId,
        );
        return previous ? { ...previous, ...item } : item;
      });
      return [...nextTabs, ...internalTabs];
    });
    setActiveTabKey((currentActiveKey) => {
      if (currentActiveKey && (
        items.some((item) => item.key === currentActiveKey)
        || tabsRef.current.some((item) => item.key === currentActiveKey)
      )) {
        return currentActiveKey;
      }
      return getInitialActiveKey(items, defaultActiveKey);
    });
  }, [defaultActiveKey, items, preserveInternalTabs]);

  // 受控模式：外部 activeKey 变化时同步内部激活态
  useEffect(() => {
    if (activeKey === undefined || activeKey === null) {
      return;
    }
    setActiveTabKey((currentActiveKey) => (currentActiveKey === activeKey ? currentActiveKey : activeKey));
  }, [activeKey]);

  const handleValueChange = useCallback(
    (value: string) => {
      setActiveTabKey(value);
      onChange?.(value);
    },
    [onChange],
  );

  const handleAdd = useCallback(() => {
    const tab = createTab?.();
    if (!tab || tabs.some((item) => item.key === tab.key)) {
      return;
    }

    setTabs((currentTabs) => [...currentTabs, tab]);
    setActiveTabKey(tab.key);
    onEdit?.(OperateType.Add, { key: tab.key });
    onChange?.(tab.key);
  }, [createTab, onChange, onEdit, tabs]);

  const handleClose = useCallback(
    (key: string) => {
      const tabIndex = tabs.findIndex((tab) => tab.key === key);
      const tab = tabs[tabIndex];
      if (!tab || tab.closable === false) {
        return;
      }

      const nextTabs = tabs.filter((item) => item.key !== key);
      const nextActiveKey =
        activeTabKey !== key
          ? activeTabKey
          : nextTabs[tabIndex]?.key ?? nextTabs[tabIndex - 1]?.key;

      setTabs(nextTabs);
      if (activeTabKey === key) {
        setActiveTabKey(nextActiveKey);
        onChange?.(nextActiveKey);
      }
      onEdit?.(OperateType.Remove, { key, remainingTabs: nextTabs });
    },
    [activeTabKey, onChange, onEdit, tabs],
  );

  useEffect(() => {
    if (!closeTabRequest || closeTabRequest.id === processedCloseRequestId.current) {
      return;
    }

    processedCloseRequestId.current = closeTabRequest.id;
    handleClose(closeTabRequest.key);
  }, [closeTabRequest, handleClose]);

  const panels = useMemo(
    () => tabs.map((tab) => ({
      instanceKey: tab.stableId ?? tab.key,
      key: tab.key,
      children: tab.children,
    })),
    [tabs],
  );

  return (
    <Tabs
      className={cn("w-full gap-0 h-full", className)}
      onValueChange={(value) => {
        if (value !== null && value !== undefined) {
          handleValueChange(String(value));
        }
      }}
      value={activeTabKey ?? null}
    >
      <EditableTabList
        hideAdd={hideAdd}
        onAdd={handleAdd}
        onClose={handleClose}
        renderAddTrigger={renderAddTrigger}
        renderTrailingAction={renderTrailingAction}
        tabs={tabs}
        className="flex-none"
      />
      <div className="relative min-h-0 flex-1 overflow-hidden">
        {panels.map((panel) => {
          const isActive = activeTabKey === panel.key;

          return (
            <TabsContent
              className={cn(
                keepMounted
                  ? isActive
                    ? "relative h-full w-full"
                    : "absolute inset-0 z-0 h-full w-full invisible pointer-events-none"
                  : "h-1/2 w-full flex-1",
              )}
              key={panel.instanceKey}
              keepMounted={keepMounted}
              hidden={keepMounted ? false : undefined}
              aria-hidden={keepMounted ? !isActive : undefined}
              value={panel.key}
            >
              <MotionReveal preset="fade" disabled={!isActive} replayKey={isActive ? "active" : "cached"} className="h-full min-h-0">
              {provideTabVisibility ? (
                <TabVisibilityProvider visible={isActive}>
                  {panel.children}
                </TabVisibilityProvider>
              ) : (
                panel.children
              )}
              </MotionReveal>
            </TabsContent>
          );
        })}
      </div>
    </Tabs>
  );
}
