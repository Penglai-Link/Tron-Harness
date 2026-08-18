import { useTranslation } from "react-i18next";
import { MousePointerClick, PanelsTopLeft, X } from "lucide-react";
import type { ReactNode } from "react";
import { useLocation } from "react-router";

import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import { cn } from "@/shared/lib/utils";
import { TabBar } from "@/shared/ui/TabBar";
import { useTabStore } from "@/store/tabStore";

interface IPanelProps {
  children: ReactNode;
  contentClassName?: string;
  onRightPanelOpenChange?: (open: boolean) => void;
  rightPanel?: ReactNode;
}

function EmptyTabWorkspace() {
  const { t } = useTranslation("shell");
  const guides = [
    {
      description: t("emptyWorkspace.guides.open.description"),
      icon: MousePointerClick,
      label: t("emptyWorkspace.guides.open.label"),
      marker: t("emptyWorkspace.guides.open.marker"),
    },
    {
      description: t("emptyWorkspace.guides.switch.description"),
      icon: PanelsTopLeft,
      label: t("emptyWorkspace.guides.switch.label"),
      marker: t("emptyWorkspace.guides.switch.marker"),
    },
    {
      description: t("emptyWorkspace.guides.close.description"),
      icon: X,
      label: t("emptyWorkspace.guides.close.label"),
      marker: t("emptyWorkspace.guides.close.marker"),
    },
  ];

  return (
    <section
      aria-labelledby="empty-tab-workspace-heading"
      className="flex min-h-full items-center justify-center px-2 py-10 sm:px-8"
      data-testid="tron-empty-tab-workspace"
    >
      <div className="w-full max-w-xl animate-in fade-in-0 duration-300">
        <p className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
          {t("emptyWorkspace.eyebrow")}
        </p>
        <h1
          className="mt-3 text-2xl font-semibold text-[var(--color-on-surface)]"
          id="empty-tab-workspace-heading"
        >
          {t("emptyWorkspace.title")}
        </h1>
        <p className="mt-3 max-w-lg text-sm leading-6 text-[var(--color-on-surface-variant)]">
          {t("emptyWorkspace.description")}
        </p>

        <div className="mt-9 border-y border-[var(--color-outline-variant)]">
          {guides.map(({ description, icon: Icon, label, marker }) => (
            <div className="flex items-center gap-4 py-4" key={label}>
              <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
                <Icon size={17} aria-hidden="true" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-[var(--color-on-surface)]">
                  {label}
                </p>
                <p className="mt-0.5 text-xs leading-5 text-[var(--color-on-surface-variant)]">
                  {description}
                </p>
              </div>
              <kbd className="shrink-0 rounded border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-2 py-1 text-xs font-medium text-[var(--color-on-surface-variant)]">
                {marker}
              </kbd>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

export function Panel({
  children,
  contentClassName,
  onRightPanelOpenChange,
  rightPanel,
}: IPanelProps) {
  const { pathname, search } = useLocation();
  const hasOpenTabs = useTabStore((state) => state.tabs.length > 0);
  const hasRightPanel = Boolean(rightPanel);
  const {
    bindRightPanelHandleRef,
    handleRightPanelPointerDown,
    handleRightPanelPointerEnd,
    handleRightPanelPointerMove,
    panelRef,
    panelResize,
    panelOpen,
  } = usePanel({
    onOpenChange: onRightPanelOpenChange,
    resetKey: `${pathname}${search}`,
  });
  return (
    <section className="flex h-full flex-col">
      <TabBar className="h-10 flex-initial" />
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
          {hasRightPanel ? (
            <>
              <ResizableHandle
                elementRef={bindRightPanelHandleRef}
                withHandle
                disabled={!panelOpen}
                className={cn(
                  "hidden lg:flex",
                  !panelOpen && "pointer-events-none opacity-0",
                )}
                onPointerDown={handleRightPanelPointerDown}
                onPointerMove={handleRightPanelPointerMove}
                onPointerUp={handleRightPanelPointerEnd}
                onPointerCancel={handleRightPanelPointerEnd}
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
                  {rightPanel}
                </aside>
              </ResizablePanel>
            </>
          ) : null}
        </ResizablePanelGroup>
      </div>
    </section>
  );
}
