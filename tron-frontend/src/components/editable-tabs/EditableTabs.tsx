import { useCallback, useEffect, useMemo, useState } from "react";

import { Tabs, TabsContent } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

import { EditableTabList } from "./EditableTabList";
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
  defaultActiveKey,
  createTab,
  hideAdd = false,
  renderAddTrigger,
  onChange,
  onEdit,
  className,
  keepMounted,
}: EditableTabsProps) {
  const [tabs, setTabs] = useState<EditableTab[]>(() => [...items]);
  const [activeKey, setActiveKey] = useState<string | undefined>(() =>
    getInitialActiveKey(items, defaultActiveKey),
  );

  useEffect(() => {
    setTabs([...items]);
    setActiveKey((currentActiveKey) => {
      if (currentActiveKey && items.some((item) => item.key === currentActiveKey)) {
        return currentActiveKey;
      }
      return getInitialActiveKey(items, defaultActiveKey);
    });
  }, [defaultActiveKey, items]);

  const handleValueChange = useCallback(
    (value: string) => {
      setActiveKey(value);
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
    setActiveKey(tab.key);
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
        activeKey !== key
          ? activeKey
          : nextTabs[tabIndex]?.key ?? nextTabs[tabIndex - 1]?.key;

      setTabs(nextTabs);
      if (activeKey === key) {
        setActiveKey(nextActiveKey);
        onChange?.(nextActiveKey);
      }
      onEdit?.(OperateType.Remove, { key, remainingTabs: nextTabs });
    },
    [activeKey, onChange, onEdit, tabs],
  );

  const panels = useMemo(
    () => tabs.map((tab) => ({ key: tab.key, children: tab.children })),
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
      value={activeKey ?? null}
    >
      <EditableTabList
        hideAdd={hideAdd}
        onAdd={handleAdd}
        onClose={handleClose}
        renderAddTrigger={renderAddTrigger}
        tabs={tabs}
        className="flex-none"
      />
      {panels.map((panel) => (
        <TabsContent
          className="w-full flex-1 h-1/2"
          key={panel.key}
          keepMounted={keepMounted}
          value={panel.key}
        >
          {panel.children}
        </TabsContent>
      ))}
    </Tabs>
  );
}
