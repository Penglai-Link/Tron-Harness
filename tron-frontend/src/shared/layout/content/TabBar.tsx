import { useRef, useState } from "react";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/shared/lib/utils";
import { useTabStore, type TabItem } from "@/store/tabStore";
import { Button } from "@/components/ui/button";

interface TabBarProps {
  className?: string;
}

export function TabBar({ className }: TabBarProps) {
  const { t } = useTranslation("shell");
  const navigate = useNavigate();
  const tabs = useTabStore((state) => state.tabs).filter((tab) => !tab.hide);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const closeTab = useTabStore((state) => state.closeTab);
  const removeTabs = useTabStore((state) => state.removeTabs);
  const refreshTab = useTabStore((state) => state.refreshTab);
  const setActiveTab = useTabStore((state) => state.setActiveTab);

  const handleTabClick = (tab: TabItem) => {
    setActiveTab(tab.id);
    navigate(tab.path);
  };

  const handleTabClose = (tabId: string) => {
    const redirect = closeTab(tabId);
    if (redirect !== null) {
      navigate(redirect);
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
        "inline-flex w-full items-center gap-1 overflow-hidden p-1",
        className,
      )}
      role="tablist"
    >
      {tabs.map((tab) => {
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
          <TabBarItem
            key={tab.id}
            tab={tab}
            isActive={tab.id === activeTabId}
            closableLeftTabs={closableLeftTabs}
            closableRightTabs={closableRightTabs}
            closableOtherTabs={closableOtherTabs}
            onActivate={() => handleTabClick(tab)}
            onClose={() => handleTabClose(tab.id)}
            onBatchClose={(tabIds) => closeTabsAndActivate(tabIds, tab)}
            onRefresh={() => refreshTab(tab.id)}
          />
        );
      })}
    </div>
  );
}

interface TabBarItemProps {
  tab: TabItem;
  isActive: boolean;
  closableLeftTabs: TabItem[];
  closableRightTabs: TabItem[];
  closableOtherTabs: TabItem[];
  onActivate: () => void;
  onClose: () => void;
  onBatchClose: (tabIds: string[]) => void;
  onRefresh: () => void;
}

function TabBarItem({
  tab,
  isActive,
  closableLeftTabs,
  closableRightTabs,
  closableOtherTabs,
  onActivate,
  onClose,
  onBatchClose,
  onRefresh,
}: TabBarItemProps) {
  const { t } = useTranslation("shell");
  const labelRef = useRef<HTMLSpanElement>(null);
  const [tooltipOpen, setTooltipOpen] = useState(false);

  const label = tab.titleTranslationKey ? t(tab.titleTranslationKey) : tab.title;

  const handleTooltipOpenChange = (open: boolean) => {
    if (!open) {
      setTooltipOpen(false);
      return;
    }
    const labelEl = labelRef.current;
    setTooltipOpen(!!labelEl && labelEl.scrollWidth > labelEl.clientWidth);
  };

  return (
    <ContextMenu>
      <Tooltip open={tooltipOpen} onOpenChange={handleTooltipOpenChange}>
        <TooltipTrigger
          delay={300}
          render={
            <ContextMenuTrigger
              className={cn(
                "group relative inline-flex h-8 min-w-0 max-w-[200px] shrink cursor-pointer items-center rounded-[4px] bg-[var(--color-surface-container)] px-1 text-sm font-medium whitespace-nowrap text-[var(--color-on-surface-variant)] transition-all",
                isActive
                  ? "bg-[var(--color-elevated-surface)] text-foreground"
                  : "hover:bg-background/50 hover:text-foreground",
              )}
              role="tab"
              aria-selected={isActive}
              onClick={onActivate}
            >
              <span
                ref={labelRef}
                className="min-w-0 flex-1 truncate px-1.5 text-left"
              >
                {label}
              </span>
              {tab.closable && (
                <Button
                  size="icon-sm"
                  variant="ghost"
                  className="shrink-0"
                  onClick={(event) => {
                    event.stopPropagation();
                    onClose();
                  }}
                >
                  <X size={12} aria-hidden="true" />
                </Button>
              )}
            </ContextMenuTrigger>
          }
        />
        <TooltipContent>{label}</TooltipContent>
      </Tooltip>
      <ContextMenuContent>
        <ContextMenuItem
          disabled={closableOtherTabs.length === 0}
          onClick={() =>
            onBatchClose(closableOtherTabs.map((item) => item.id))
          }
        >
          {t("tabs.closeOthers")}
        </ContextMenuItem>
        <ContextMenuItem
          disabled={closableLeftTabs.length === 0}
          onClick={() => onBatchClose(closableLeftTabs.map((item) => item.id))}
        >
          {t("tabs.closeLeft")}
        </ContextMenuItem>
        <ContextMenuItem
          disabled={closableRightTabs.length === 0}
          onClick={() => onBatchClose(closableRightTabs.map((item) => item.id))}
        >
          {t("tabs.closeRight")}
        </ContextMenuItem>
        <ContextMenuItem onClick={onRefresh}>
          {t("tabs.refresh")}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
