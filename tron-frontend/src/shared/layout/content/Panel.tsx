import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Command } from "lucide-react";
import type { ReactNode } from "react";
import { useLocation } from "react-router";

import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import {
  formatShortcutKeys,
  shellShortcutActions,
} from "@/shared/lib/keyboardShortcuts";
import { cn } from "@/shared/lib/utils";
import { TabBar } from "@/shared/ui/TabBar";
import { useAppShellStore } from "@/store/appShellStore";
import { useTabStore } from "@/store/tabStore";
import RightPanelContainer from "./RightPanelContainer";

interface IPanelProps {
  children: ReactNode;
  contentClassName?: string;
}

function EmptyTabWorkspace() {
  const { t: pageT } = useTranslation("pages");
  const shortcutBindings = useAppShellStore((state) => state.shortcutBindings);
  const useMacSymbols =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/i.test(navigator.userAgent);
  const shortcuts = shellShortcutActions.flatMap((action) => {
    const binding = shortcutBindings[action];
    if (!binding) return [];

    return [
      {
        action,
        keys: formatShortcutKeys(binding, useMacSymbols),
      },
    ];
  });

  return (
    <section
      aria-label={pageT("settingsShortcuts.title")}
      className="flex min-h-full items-center justify-center overflow-hidden px-6 py-10"
      data-testid="tron-empty-tab-workspace"
    >
      <div className="flex w-full max-w-md flex-col items-center animate-in fade-in-0 slide-in-from-bottom-2 duration-300">
        <Command
          aria-hidden="true"
          className="mb-10 size-40 text-[var(--color-surface-container)] sm:size-52"
          strokeWidth={1}
        />
        <div
          className="w-full space-y-2.5"
          data-testid="tron-empty-tab-shortcuts"
        >
          {shortcuts.map(({ action, keys }) => (
            <div
              className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-5"
              key={action}
            >
              <span className="text-sm text-[var(--color-on-surface-variant)]">
                {pageT(`settingsShortcuts.shortcuts.${action}.label`)}
              </span>
              <span
                className="flex items-center gap-1"
                aria-label={keys.join(" ")}
              >
                {keys.map((key, index) => (
                  <kbd
                    className="min-w-6 rounded border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-1.5 py-0.5 text-center text-xs font-medium text-[var(--color-on-surface-variant)] shadow-[0_1px_0_rgba(15,23,42,0.05)]"
                    key={`${key}-${index}`}
                  >
                    {key}
                  </kbd>
                ))}
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

export function Panel({ children, contentClassName }: IPanelProps) {
  const { pathname, search } = useLocation();
  const hasOpenTabs = useTabStore((state) => state.tabs.length > 0);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const activeTab = useTabStore((state) =>
    state.tabs.find((tab) => tab.id === state.activeTabId),
  );
  const updateRightPanelState = useTabStore(
    (state) => state.updateRightPanelState,
  );
  const storedPanelOpen = Boolean(activeTab?.rightPanelOpen);
  const handleRightPanelOpenChange = useCallback(
    (open: boolean) => {
      if (activeTabId) {
        updateRightPanelState(activeTabId, { open });
      }
    },
    [activeTabId, updateRightPanelState],
  );
  const handleRightPanelSizeChange = useCallback(
    (size: number) => {
      if (activeTabId && Number.isFinite(size) && size > 0) {
        updateRightPanelState(activeTabId, { size: Math.round(size) });
      }
    },
    [activeTabId, updateRightPanelState],
  );
  const {
    handlePanelPointerDown,
    handlePanelPointerEnd,
    handlePanelPointerMove,
    panelOpen,
    panelRef,
    panelResize,
  } = usePanel({
    open: storedPanelOpen,
    initialSize: activeTab?.rightPanelSize,
    onOpenChange: handleRightPanelOpenChange,
    onSizeChange: handleRightPanelSizeChange,
    resetKey: activeTabId ?? `${pathname}${search}`,
  });
  return (
    <section className="flex h-full flex-col">
      <TabBar className="h-12 flex-initial" />
      <div className={cn("relative z-0 grid h-1/2 flex-1 grid-cols-1")}>
        <style>{`
          #resize-panel {
            display: none !important;
          }

          @media (min-width: 1024px) {
            #resize-panel {
              display: flex !important;
            }
          }
        `}</style>
        <ResizablePanelGroup orientation="horizontal" className="w-full">
          <ResizablePanel defaultSize="100%">
            <main
              className={cn(
                "relative z-0 h-full min-w-0 overflow-x-hidden overflow-y-auto px-6 py-6",
                contentClassName,
              )}
            >
              {hasOpenTabs ? children : <EmptyTabWorkspace />}
            </main>
          </ResizablePanel>
          <>
            <ResizableHandle
              withHandle
              className={!panelOpen ? 'opacity-0' : ''}
              disabled={!panelOpen}
              onPointerDown={handlePanelPointerDown}
              onPointerMove={handlePanelPointerMove}
              onPointerUp={handlePanelPointerEnd}
              onPointerCancel={handlePanelPointerEnd}
            />
            <ResizablePanel
              panelRef={panelRef}
              id="resize-panel"
              collapsible
              collapsedSize={0}
              defaultSize="0%"
              minSize={400}
              maxSize="80%"
              onResize={panelResize}
            >
              <aside
                className="relative z-10 hidden h-full overflow-y-auto bg-[var(--color-elevated-surface)] lg:block"
                data-testid="tron-right-sidebar"
              >
                <RightPanelContainer />
              </aside>
            </ResizablePanel>
          </>
        </ResizablePanelGroup>
      </div>
    </section>
  );
}
