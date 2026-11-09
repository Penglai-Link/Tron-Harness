import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { PanelImperativeHandle } from "react-resizable-panels";

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { useSidebar } from "@/components/ui/sidebar";
import { useAppShellStore } from "@/store/appShellStore";

interface WorkspaceLayoutProps {
  navigation: ReactNode;
  toolbar: ReactNode;
  children: ReactNode;
}

function expandedNavigationWidth(width: number) {
  const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
  return Math.min(20 * rem, Math.max(12 * rem, width));
}

function DesktopWorkspaceBase({ navigation, toolbar, onNavigationResize }: Omit<WorkspaceLayoutProps, "children"> & {
  onNavigationResize: (width: number) => void;
}) {
  const { open, setOpen } = useSidebar();
  const { t } = useTranslation();
  const panelRef = useRef<PanelImperativeHandle>(null);
  const groupElementRef = useRef<HTMLDivElement>(null);
  const panelElementRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef(false);
  const savedWidth = useAppShellStore((state) => state.sidebarWidth);
  const setSidebarWidth = useAppShellStore((state) => state.setSidebarWidth);
  const [expandedSize] = useState(() => {
    const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
    return savedWidth === null
      ? Math.min(14 * rem, Math.max(12 * rem, window.innerWidth * 0.16))
      : expandedNavigationWidth(savedWidth);
  });
  const [defaultSize] = useState(() => open ? expandedSize : "3.5rem");

  useEffect(() => {
    const endDrag = () => { draggingRef.current = false; };
    window.addEventListener("pointerup", endDrag);
    window.addEventListener("pointercancel", endDrag);
    return () => {
      window.removeEventListener("pointerup", endDrag);
      window.removeEventListener("pointercancel", endDrag);
    };
  }, []);

  // External shortcuts and direct resizing share the SidebarProvider state.
  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;
    if (!open) panel.collapse();
    else if (panel.isCollapsed()) panel.resize(expandedNavigationWidth(savedWidth ?? expandedSize));
  }, [expandedSize, open, savedWidth]);

  return (
    <ResizablePanelGroup
      id="tron-shell-base"
      elementRef={groupElementRef}
      className="tron-shell-base"
      orientation="horizontal"
      resizeTargetMinimumSize={{ fine: 12, coarse: 24 }}
      onLayoutChanged={(layout, { isUserInteraction }) => {
        const panel = panelRef.current;
        if (!isUserInteraction || !panel) return;
        setOpen(!panel.isCollapsed());
        if (!panel.isCollapsed() && groupElementRef.current) {
          // Keyboard callbacks precede the DOM update: use the new layout,
          // rather than persisting the previous (possibly collapsed) width.
          setSidebarWidth(layout["tron-navigation-panel"] / 100 * groupElementRef.current.clientWidth);
        }
      }}
    >
      <ResizablePanel
        id="tron-navigation-panel"
        className="flex min-w-0"
        style={{ overflow: "visible" }}
        panelRef={panelRef}
        elementRef={panelElementRef}
        collapsible
        collapsedSize="3.5rem"
        defaultSize={defaultSize}
        minSize="12rem"
        maxSize="20rem"
        groupResizeBehavior="preserve-pixel-size"
        onResize={({ inPixels }) => {
          onNavigationResize(panelElementRef.current?.getBoundingClientRect().width ?? inPixels);
          const panel = panelRef.current;
          // ResizeObserver also fires after shortcuts and viewport changes. Only
          // a live drag may feed its collapsed state back into the sidebar.
          if (draggingRef.current && panel && panel.isCollapsed() === open) setOpen(!panel.isCollapsed());
        }}
      >
        {navigation}
      </ResizablePanel>
      <ResizableHandle
        id="tron-navigation-resizer"
        className="tron-navigation-resizer"
        disableDoubleClick
        onPointerDown={() => { draggingRef.current = true; }}
        aria-label={t("shell:aria.resizeSidebar")}
        aria-valuetext={t(open ? "shell:aria.sidebarExpanded" : "shell:aria.sidebarCollapsed")}
        title={t("shell:aria.resizeSidebarHint")}
      />
      <ResizablePanel id="tron-toolbar-panel" minSize="50%" style={{ overflow: "visible" }}>
        {toolbar}
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

export function WorkspaceLayout({ navigation, toolbar, children }: WorkspaceLayoutProps) {
  const { isMobile, open } = useSidebar();
  const [navigationWidth, setNavigationWidth] = useState<number>();

  return (
    <div
      className="tron-workspace-layout"
      style={{ "--tron-navigation-width": isMobile ? "0px" : navigationWidth === undefined ? (open ? undefined : "3.5rem") : `${navigationWidth}px` } as CSSProperties}
    >
      {isMobile ? (
        <div className="tron-shell-base" data-testid="tron-shell-base">
          {toolbar}
          {navigation}
        </div>
      ) : (
        <DesktopWorkspaceBase navigation={navigation} toolbar={toolbar} onNavigationResize={setNavigationWidth} />
      )}
      <div className="tron-shell-workspace" data-theme-part="app-content">
        <div className="tron-workspace-surface" data-testid="tron-workspace-surface">
          {children}
        </div>
      </div>
    </div>
  );
}
