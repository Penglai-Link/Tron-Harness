import { X } from "lucide-react";
import { useNavigate } from "react-router";
import { cn } from "@/shared/lib/utils";
import { useTabStore, type TabItem } from "@/store/tabStore";
import { Button } from "@/components/ui/button";

interface TabBarProps {
  className?: string;
}

export function TabBar({ className }: TabBarProps) {
  const navigate = useNavigate();
  const tabs = useTabStore((state) => state.tabs);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const removeTab = useTabStore((state) => state.removeTab);
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
        return (
          <div
            key={tab.id}
            className={cn(
              "group relative text-[var(--color-on-surface-variant)] inline-flex h-8 max-w-[200px] shrink-0 items-center rounded-md px-1 text-sm font-medium whitespace-nowrap transition-all cursor-pointer",
              isActive
                ? "bg-[var(--color-elevated-surface)] text-foreground"
                : "hover:text-foreground hover:bg-background/50",
            )}
            role="tab"
            aria-selected={isActive}
            onClick={() => handleTabClick(tab)}
          >
            <span className="truncate flex-1 text-left px-1.5">{tab.title}</span>
            {tab.closable && (
              <Button
                size="icon-sm"
                variant="ghost"
                onClick={(e) => handleTabClose(e, tab.id)}
              >
                <X size={12} aria-hidden="true" />
              </Button>
            )}
          </div>
        );
      })}
    </div>
  );
}
