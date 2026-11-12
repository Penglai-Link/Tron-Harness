import { useRef, type ComponentProps } from "react";

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
import { X } from "lucide-react";
import { ShortcutHint } from "@/shared/shortcuts/ShortcutHints";
import type { ShellShortcutAction } from "@/shared/lib/keyboardShortcuts";

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
    const hasVisibleTabs = useTabStore
      .getState()
      .tabs.some((tab) => !tab.hide);
    if (hasVisibleTabs && redirect !== null) {
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
        "tron-scrollbar-none flex w-full min-w-0 items-center gap-1 overflow-x-auto px-1 py-2",
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
            shortcutAction={tabIndex < 9 ? `activateTab${tabIndex + 1}` as ShellShortcutAction : undefined}
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
  shortcutAction?: ShellShortcutAction;
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
  shortcutAction,
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

  const label = tab.titleTranslationKey ? t(tab.titleTranslationKey) : tab.title;

  const handleTooltipOpenChange: NonNullable<ComponentProps<typeof Tooltip>["onOpenChange"]> = (open, details) => {
    const labelEl = labelRef.current;
    if (open && (!labelEl || labelEl.scrollWidth <= labelEl.clientWidth)) details.cancel();
  };

  return (
    <ContextMenu>
      <Tooltip onOpenChange={handleTooltipOpenChange}>
        <TooltipTrigger
          delay={300}
          render={
            <ContextMenuTrigger
              className={cn(
                "group relative inline-flex h-8 min-w-0 max-w-[16rem] shrink-0 cursor-pointer items-center rounded-lg px-1 text-sm font-medium whitespace-nowrap text-[var(--color-on-surface-variant)] transition-all",
                isActive
                  ? "bg-[var(--tron-tab-active-bg,var(--color-elevated-surface))] text-foreground"
                  : "hover:bg-background/50 hover:text-foreground",
              )}
              role="tab"
              aria-label={label}
              aria-selected={isActive}
              onClick={onActivate}
            >
              <span
                ref={labelRef}
                className="min-w-0 flex-1 truncate px-1.5 text-left"
              >
                {label}
              </span>
              {shortcutAction ? <ShortcutHint action={shortcutAction} compact align="start" /> : null}
              {tab.closable && (
                <Button
                  size="icon-xs"
                  variant="ghost"
                  className="shrink-0"
                  aria-label={t("common:actions.close", "Close")}
                  title={t("common:actions.close", "Close")}
                  onClick={(event) => {
                    event.stopPropagation();
                    onClose();
                  }}
                >
                  <X aria-hidden="true" className="size-3.5" />
                  {isActive ? <ShortcutHint action="closeCurrentTab" compact /> : null}
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
          <ShortcutHint action="closeOtherTabs" />
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
