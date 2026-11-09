import { useCallback } from "react";
import type { ReactNode } from "react";

import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import { cn } from "@/shared/lib/utils";
import { DEFAULT_TAB_ID, useTabStore } from "@/store/tabStore";
import RightPanelContainer from "./RightPanelContainer";
import { EmptyTabWorkspace } from "./EmptyTabWorkspace";

interface IPanelProps {
  children: ReactNode;
  contentClassName?: string;
}

export function Panel({ children, contentClassName }: IPanelProps) {
  const tabs = useTabStore((state) => state.tabs).filter((tab) => !tab.hide);
  const hasOpenTabs = tabs.length > 0;
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);

  const activeTab = useTabStore((state) =>
    state.tabs.find((tab) => tab.id === (state.activeTabId ?? DEFAULT_TAB_ID)),
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
    adaptToWidth: true,
    open: storedPanelOpen,
    initialSize: activeTab?.rightPanelSize,
    onOpenChange: handleRightPanelOpenChange,
    onSizeChange: handleRightPanelSizeChange,
    resetKey: activeTabId,
  });
  return (
    <section className="flex h-full flex-col">
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
          <ResizablePanel defaultSize="100%" minSize="40%" className="min-w-0" style={{ overflow: "hidden" }}>
            <main
              className={cn(
                "relative z-0 h-full min-w-0 overflow-x-hidden overflow-y-auto bg-[var(--color-elevated-surface)] px-4 py-4 md:px-6 md:py-6 text-[var(--color-on-surface)]",
                contentClassName,
              )}
              data-theme-part="app-page"
              data-theme-anchor="page"
            >
              <div className="h-full" hidden={!hasOpenTabs}>{children}</div>
              {!hasOpenTabs ? <EmptyTabWorkspace /> : null}
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
              minSize="25%"
              maxSize="60%"
              onResize={panelResize}
            >
              <aside
                className="relative z-10 hidden h-full overflow-y-auto bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] lg:block"
                data-theme-part="app-right-sidebar"
                data-theme-anchor="panel"
                data-testid="tron-right-sidebar"
                aria-hidden={!panelOpen}
                inert={!panelOpen}
                style={{ visibility: panelOpen ? "visible" : "hidden" }}
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
