import { X } from "lucide-react";
import { useNavigate } from "react-router";
import { cn } from "@/shared/lib/utils";
import { useTabStore, type TabItem } from "@/store/tabStore";

interface TabBarProps {
  className?: string;
}

export function TabBar({ className }: TabBarProps) {
  const navigate = useNavigate();
  const tabs = useTabStore((state) => state.tabs);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const removeTab = useTabStore((state) => state.removeTab);
  const setActiveTab = useTabStore((state) => state.setActiveTab);

  const handleTabClick = (tab: TabItem) => {
    setActiveTab(tab.id);
    navigate(tab.path);
  };

  const handleTabClose = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    removeTab(tabId);
  };

  if (tabs.length === 0) {
    return null;
  }

  return (
    <div
      className={cn(
        "inline-flex w-full items-center gap-1 overflow-x-auto bg-[var(--color-surface-container)] p-1 border-b-[1px] border-[var(--color-card-border)]",
        className
      )}
      role="tablist"
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        return (
          <div
            key={tab.id}
            className={cn(
              "group relative text-[var(--color-on-surface-variant)] inline-flex h-8 max-w-[200px] shrink-0 items-center gap-1.5 rounded-md px-3 text-sm font-medium whitespace-nowrap transition-all cursor-pointer",
              isActive
                ? "bg-[var(--color-elevated-surface)] text-foreground"
                : "hover:text-foreground hover:bg-background/50"
            )}
            role="tab"
            aria-selected={isActive}
            onClick={() => handleTabClick(tab)}
          >
            <span className="truncate flex-1 text-left">{tab.title}</span>
            {tab.closable && (
              <button
                className={cn(
                  "inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-sm transition-opacity",
                  "hover:text-foreground",
                  "opacity-0 group-hover:opacity-100",
                  isActive && "opacity-60"
                )}
                type="button"
                aria-label={`Close ${tab.title}`}
                onClick={(e) => handleTabClose(e, tab.id)}
              >
                <X className="size-3" aria-hidden="true" />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
