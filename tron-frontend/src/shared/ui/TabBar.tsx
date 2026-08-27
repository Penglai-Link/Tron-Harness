import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { cn } from "@/shared/lib/utils";
import { useTabStore, type TabItem } from "@/store/tabStore";
import { Button } from "@/components/ui/button";

interface TabBarProps {
  className?: string;
}

export function TabBar({ className }: TabBarProps) {
  const { t } = useTranslation("shell");
  const navigate = useNavigate();
  const tabs = useTabStore((state) => state.tabs);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const removeTab = useTabStore((state) => state.removeTab);
  const removeTabs = useTabStore((state) => state.removeTabs);
  const refreshTab = useTabStore((state) => state.refreshTab);
  const setActiveTab = useTabStore((state) => state.setActiveTab);
  const suppressNextTab = useTabStore((state) => state.suppressNextTab);

  const handleTabClick = (tab: TabItem) => {
    setActiveTab(tab.id);
    navigate(tab.path);
  };

  const handleTabClose = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    const closingActiveTab = tabId === activeTabId;
    const closingIndex = tabs.findIndex((tab) => tab.id === tabId);
    const remainingTabs = tabs.filter((tab) => tab.id !== tabId);
    const nextTab = closingActiveTab
      ? remainingTabs[Math.min(closingIndex, remainingTabs.length - 1)]
      : undefined;

    removeTab(tabId);
    if (closingActiveTab) {
      if (!nextTab) {
        suppressNextTab();
      }
      navigate(nextTab?.path ?? "/apps");
    }
  };

  const closeTabsAndActivate = (
    tabIds: readonly string[],
    activeTab: TabItem,
  ) => {
    if (tabIds.length === 0) return;
    removeTabs(tabIds);
    setActiveTab(activeTab.id);
    navigate(activeTab.path);
  };

  if (tabs.length === 0) {
    return null;
  }

  return (
    <div
      className={cn(
        "inline-flex w-full items-center gap-1 overflow-x-auto bg-[var(--color-surface-container)] p-1 border-b-[1px] border-[var(--color-card-border)]",
        className,
      )}
      role="tablist"
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        const tabIndex = tabs.findIndex((item) => item.id === tab.id);
        const closableLeftTabs = tabs
          .slice(0, tabIndex)
          .filter((item) => item.closable);
        const closableRightTabs = tabs
          .slice(tabIndex + 1)
          .filter((item) => item.closable);
        const closableOtherTabs = tabs.filter(
          (item) => item.id !== tab.id && item.closable,
        );
        return (
          <ContextMenu key={tab.id}>
            <ContextMenuTrigger
              className={cn(
                "group relative inline-flex h-8 max-w-[200px] shrink-0 cursor-pointer items-center rounded-md px-1 text-sm font-medium whitespace-nowrap text-[var(--color-on-surface-variant)] transition-all",
                isActive
                  ? "bg-[var(--color-elevated-surface)] text-foreground"
                  : "hover:bg-background/50 hover:text-foreground",
              )}
              role="tab"
              aria-selected={isActive}
              onClick={() => handleTabClick(tab)}
            >
              <span className="flex-1 truncate px-1.5 text-left">
                {tab.titleTranslationKey ? t(tab.titleTranslationKey) : tab.title}
              </span>
              {tab.closable && (
                <Button
                  size="icon-sm"
                  variant="ghost"
                  onClick={(event) => handleTabClose(event, tab.id)}
                >
                  <X size={12} aria-hidden="true" />
                </Button>
              )}
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem
                disabled={closableOtherTabs.length === 0}
                onClick={() =>
                  closeTabsAndActivate(
                    closableOtherTabs.map((item) => item.id),
                    tab,
                  )
                }
              >
                {t("tabs.closeOthers")}
              </ContextMenuItem>
              <ContextMenuItem
                disabled={closableLeftTabs.length === 0}
                onClick={() =>
                  closeTabsAndActivate(
                    closableLeftTabs.map((item) => item.id),
                    tab,
                  )
                }
              >
                {t("tabs.closeLeft")}
              </ContextMenuItem>
              <ContextMenuItem
                disabled={closableRightTabs.length === 0}
                onClick={() =>
                  closeTabsAndActivate(
                    closableRightTabs.map((item) => item.id),
                    tab,
                  )
                }
              >
                {t("tabs.closeRight")}
              </ContextMenuItem>
              <ContextMenuItem onClick={() => refreshTab(tab.id)}>
                {t("tabs.refresh")}
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        );
      })}
    </div>
  );
}
