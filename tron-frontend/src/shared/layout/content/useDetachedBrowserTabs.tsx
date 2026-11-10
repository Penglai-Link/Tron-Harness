import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";

import { PageView } from "@/components/page-view";
import { TabVisibilityProvider, type EditableTab } from "@/components/editable-tabs";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { DEFAULT_TAB_ID, useTabStore } from "@/store/tabStore";
import { useAppShellStore } from "@/store/appShellStore";
import type { TabPanelConfig } from "./TabPanelRegistry";

interface DetachedTab {
  panelId: string;
  stableId: string;
  popup: Window;
  container: HTMLDivElement;
  dispose: () => void;
  restoreLocation: () => void;
}

/** Copy styles, never scripts or application bootstrap, into the child document. */
function prepareWindow(popup: Window) {
  const target = popup.document;
  const base = target.createElement("base");
  base.href = document.baseURI;
  target.head.append(base);
  const styles = target.createElement("div");
  target.head.append(styles);
  const syncStyles = () => {
    styles.replaceChildren(...Array.from(document.querySelectorAll('style, link[rel="stylesheet"]'))
      .map((node) => node.cloneNode(true)));
  };
  const syncTheme = () => {
    for (const attribute of Array.from(target.documentElement.attributes)) {
      target.documentElement.removeAttribute(attribute.name);
    }
    for (const attribute of Array.from(document.documentElement.attributes)) {
      target.documentElement.setAttribute(attribute.name, attribute.value);
    }
    target.body.className = document.body.className;
  };
  syncStyles();
  syncTheme();
  const observer = new MutationObserver(syncStyles);
  observer.observe(document.head, { subtree: true, childList: true, characterData: true, attributes: true });
  const themeObserver = new MutationObserver(syncTheme);
  themeObserver.observe(document.documentElement, { attributes: true });
  themeObserver.observe(document.body, { attributes: true });
  const container = target.createElement("div");
  container.style.cssText = "height:100vh;width:100%;overflow:hidden";
  target.body.style.margin = "0";
  target.body.append(container);
  return { container, dispose: () => { observer.disconnect(); themeObserver.disconnect(); } };
}

export function useDetachedBrowserTabs(
  tabs: Record<string, readonly EditableTab[]>,
  configs: Record<string, TabPanelConfig>,
  ownerKeys: Record<string, string | undefined>,
  update: (panelId: string, stableId: string, changes: Partial<EditableTab>) => void,
  activate: (panelId: string, key: string) => void,
  setOwner: (panelId: string, ownerKey: string | undefined) => void,
) {
  const { t } = useTranslation("common");
  const toast = useToast();
  const navigate = useNavigate();
  const [windows, setWindows] = useState<DetachedTab[]>([]);
  const windowsRef = useRef(windows);
  windowsRef.current = windows;

  const returnTab = useCallback((stableId: string, reveal = true) => {
    const entry = windowsRef.current.find((item) => item.stableId === stableId);
    if (!entry) return;
    const tab = tabs[entry.panelId]?.find((item) => item.stableId === stableId);
    entry.dispose();
    entry.popup.close();
    windowsRef.current = windowsRef.current.filter((item) => item !== entry);
    setWindows(windowsRef.current);
    if (tab) {
      setOwner(entry.panelId, tab.ownerKey);
      activate(entry.panelId, tab.key);
      if (reveal) {
        entry.restoreLocation();
        window.focus();
      }
    }
  }, [activate, setOwner, tabs]);

  const detach = useCallback((panelId: string, tab: EditableTab, position?: { screenX: number; screenY: number }) => {
    if (!tab.stableId || tab.url === undefined) return;
    const existing = windowsRef.current.find((item) => item.stableId === tab.stableId);
    if (existing) { existing.popup.focus(); return; }
    // Must stay synchronous with the click / pointer-up to retain user activation.
    const features = `popup=yes,width=1000,height=760${position ? `,left=${Math.round(position.screenX - 100)},top=${Math.round(position.screenY - 24)}` : ""}`;
    const popup = window.open("about:blank", `tron-detached-${crypto.randomUUID()}`, features);
    if (!popup) { toast.error(t("content.popupBlocked")); return; }
    let prepared: ReturnType<typeof prepareWindow>;
    try { prepared = prepareWindow(popup); }
    catch { popup.close(); toast.error(t("content.popupBlocked")); return; }
    const tabState = useTabStore.getState();
    const workspaceCode = tabState.workspaceCode;
    const hostTabId = panelId.startsWith(`${DEFAULT_TAB_ID}:`) ? DEFAULT_TAB_ID : panelId;
    const hostPath = tabState.tabs.find((item) => item.id === hostTabId)?.path ?? "/workspace";
    const { selectedPrimarySpaceId, selectedSecondarySpaceId } = useAppShellStore.getState();
    const entry: DetachedTab = {
      panelId, stableId: tab.stableId, popup, ...prepared,
      restoreLocation: () => {
        // Draft promotion and session renaming may replace the host route while
        // this window is open. Resolve it from the live tab, not the captured URL.
        const currentPath = useTabStore.getState().getAllTabs().find((item) => item.id === hostTabId)?.path ?? hostPath;
        if (workspaceCode) {
          useAppShellStore.getState().selectPrimarySpace(selectedPrimarySpaceId, selectedSecondarySpaceId);
          useTabStore.getState().switchWorkspace(workspaceCode, currentPath);
        }
        useTabStore.getState().setActiveTab(hostTabId);
        useTabStore.getState().updateRightPanelState(hostTabId, { open: true });
        navigate(hostTabId === DEFAULT_TAB_ID ? "/workspace" : currentPath);
      },
    };
    windowsRef.current = [...windowsRef.current, entry];
    setWindows(windowsRef.current);
    popup.focus();
  }, [navigate, t, toast]);

  useEffect(() => {
    if (!windows.length) return;
    // Closing/reloading the native window returns the same registry item. The
    // placeholder never leaves the array, so surrounding tab changes cannot shift it.
    const timer = window.setInterval(() => {
      for (const entry of windowsRef.current) {
        let documentChanged = false;
        try { documentChanged = entry.popup.document !== entry.container.ownerDocument; }
        catch { documentChanged = true; }
        if (entry.popup.closed || documentChanged || !entry.container.isConnected
          || !tabs[entry.panelId]?.some((item) => item.stableId === entry.stableId)) {
          returnTab(entry.stableId, false);
        }
      }
    }, 300);
    return () => window.clearInterval(timer);
  }, [returnTab, tabs, windows.length]);

  useEffect(() => {
    const dispose = () => {
      for (const entry of windowsRef.current) { entry.dispose(); entry.popup.close(); }
      windowsRef.current = [];
    };
    window.addEventListener("pagehide", dispose);
    return () => { window.removeEventListener("pagehide", dispose); dispose(); };
  }, []);

  useEffect(() => {
    for (const entry of windows) {
      const tab = tabs[entry.panelId]?.find((item) => item.stableId === entry.stableId);
      if (tab) entry.container.ownerDocument.title = `${typeof tab.label === "string" ? tab.label : t("content.browser")} — Tron`;
    }
  }, [tabs, windows, t]);
  const detachedRightPanelIds = useMemo(() => windows.map((entry) => entry.stableId), [windows]);
  const focusRightPanelWindow = useCallback((stableId: string) => {
    windowsRef.current.find((entry) => entry.stableId === stableId)?.popup.focus();
  }, []);

  const portals = windows.map((entry) => {
    const tab = tabs[entry.panelId]?.find((item) => item.stableId === entry.stableId);
    if (!tab || entry.popup.closed) return null;
    const config = configs[entry.panelId];
    // A route may reuse its host tab for another session. Never send annotations
    // from an older detached page to the newly selected session.
    const onInspectionConfirm = tab.ownerKey === ownerKeys[entry.panelId]
      ? config?.onPageInspectionConfirm : undefined;
    return createPortal(
      <TabVisibilityProvider visible>
        <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
          <header className="flex shrink-0 items-center justify-between gap-4 border-b px-3 py-2">
            <span className="min-w-0 truncate text-sm">{tab.label || t("content.newTab")}</span>
            <Button size="sm" variant="outline" onClick={() => returnTab(entry.stableId)}>
              <ArrowLeft aria-hidden="true" size={14} />{t("content.returnTab")}
            </Button>
          </header>
          <div className="min-h-0 flex-1 overflow-auto">
            <PageView pageId={entry.stableId} url={tab.url ?? ""}
              inspectionMode={Boolean(onInspectionConfirm)}
              onInspectionConfirm={onInspectionConfirm}
              onUrlChange={(url) => update(entry.panelId, entry.stableId, {
                key: `${entry.stableId}:${url}`, label: url || t("content.newTab"), url,
              })} />
          </div>
        </div>
      </TabVisibilityProvider>, entry.container, entry.stableId,
    );
  });

  return {
    detachRightPanelTab: detach,
    detachedRightPanelIds,
    returnRightPanelTab: returnTab,
    focusRightPanelWindow,
    portals,
  };
}
