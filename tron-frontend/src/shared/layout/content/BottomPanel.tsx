import { Maximize2, Minimize2, Plus, ChevronDown } from "lucide-react";
import { type Dispatch, type SetStateAction, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { EditableTabs, type EditableTab } from "@/components/editable-tabs";
import { type EditableTabCloseRequest, type EditableTabEditInfo, OperateType } from "@/components/editable-tabs/types";
import Terminal from "@/components/terminal";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { ResizableHandle, ResizablePanel } from "@/components/ui/resizable";
import { AgentTerminalTab } from "@/domain/chat/terminal/AgentTerminalPanel";
import { closeAgentTerminal, openAgentTerminal, refreshAgentTerminals, useAgentTerminals } from "@/domain/chat/terminal/store";
import { usePanel } from "@/shared/hooks/usePanel";

const agentTabKey = (sessionCode: string, id: string) => `agent-terminal:${sessionCode}:${id}`;

export default function BottomPanel({
  visible,
  changeBottomPanel,
  sessionCode,
}: {
  visible: boolean;
  changeBottomPanel?: Dispatch<SetStateAction<boolean>>;
  sessionCode?: string;
}) {
  const { t } = useTranslation("pages");
  const nextTabId = useRef(1);
  const nextCloseRequestId = useRef(1);
  const [manualTabs, setManualTabs] = useState<EditableTab[]>([]);
  const [activeKey, setActiveKey] = useState<string>();
  const [closeTabRequest, setCloseTabRequest] = useState<EditableTabCloseRequest | null>(null);
  const [maximized, setMaximized] = useState(false);
  const restoreSizeRef = useRef<number | null>(null);
  const hasToggledMaximizeRef = useRef(false);
  const session = useAgentTerminals((state) => sessionCode ? state.sessions[sessionCode] : undefined);
  const runningTerminals = session?.terminals.filter((item) => item.running) ?? [];

  const requestTabClose = useCallback((key: string) => {
    setCloseTabRequest({ id: nextCloseRequestId.current++, key });
  }, []);
  const createManualTab = useCallback((): EditableTab => {
    const key = `terminal-${nextTabId.current++}`;
    const tab: EditableTab = {
      key,
      label: "tron",
      children: <Terminal onExit={() => requestTabClose(key)} />,
      closable: true,
    };
    setManualTabs((tabs) => [...tabs, tab]);
    return tab;
  }, [requestTabClose]);

  const agentTabs = useMemo<EditableTab[]>(() => {
    if (!sessionCode) return [];
    return (session?.tabs ?? []).map((id) => {
      const info = session?.terminals.find((item) => item.terminalId === id);
      const label = info
        ? `${info.command} · ${info.cwd.split(/[\\/]/).slice(-2).join("/")}`
        : `${t("agentTerminal.title")} · ${id.slice(0, 8)}`;
      const key = agentTabKey(sessionCode, id);
      return {
        key,
        label,
        children: <AgentTerminalTab sessionCode={sessionCode} terminalId={id} visible={visible && activeKey === key} />,
        closable: true,
      };
    });
  }, [activeKey, session?.tabs, session?.terminals, sessionCode, t, visible]);
  const items = useMemo(() => [...manualTabs, ...agentTabs], [manualTabs, agentTabs]);

  useEffect(() => {
    if (!visible || manualTabs.length || agentTabs.length) return;
    const tab = createManualTab();
    setActiveKey(tab.key);
  }, [agentTabs.length, createManualTab, manualTabs.length, visible]);

  useEffect(() => {
    if (sessionCode && session?.selected) {
      setActiveKey(agentTabKey(sessionCode, session.selected));
    }
  }, [sessionCode, session?.selected]);

  useEffect(() => {
    setActiveKey((current) => current && items.some((item) => item.key === current)
      ? current
      : items[0]?.key);
  }, [items]);

  useEffect(() => {
    if (!sessionCode || !visible) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        await refreshAgentTerminals(sessionCode, controller.signal);
      } catch {
        if (!controller.signal.aborted) {
          useAgentTerminals.getState().update(sessionCode, (value) => ({
            ...value,
            error: t("agentTerminal.unavailable"),
          }));
        }
      }
      if (!controller.signal.aborted) timer = setTimeout(refresh, 2000);
    };
    void refresh();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [sessionCode, t, visible]);

  const panel = usePanel({
    collapseThreshold: 120,
    initialSize: "25%",
    open: visible,
    onOpenChange: (open) => changeBottomPanel?.(open),
  });

  useEffect(() => {
    if (!visible) {
      setMaximized(false);
      restoreSizeRef.current = null;
      hasToggledMaximizeRef.current = false;
    }
  }, [visible]);

  useEffect(() => {
    if (!visible || !hasToggledMaximizeRef.current) return;
    const frame = window.requestAnimationFrame(() => {
      panel.panelRef.current?.resize(maximized ? "100%" : restoreSizeRef.current === null ? "25%" : `${restoreSizeRef.current}px`);
    });
    return () => window.cancelAnimationFrame(frame);
  }, [maximized, panel.panelRef, visible]);

  function toggleMaximized() {
    if (!maximized) restoreSizeRef.current = panel.panelRef.current?.getSize().inPixels ?? null;
    hasToggledMaximizeRef.current = true;
    setMaximized((value) => !value);
  }

  function onChange(key: string | undefined) {
    setActiveKey(key);
  }

  function onEdit(action: OperateType, { key, remainingTabs }: EditableTabEditInfo) {
    if (action !== OperateType.Remove || !key) return;
    if (sessionCode && key.startsWith(`agent-terminal:${sessionCode}:`)) {
      closeAgentTerminal(sessionCode, key.slice(`agent-terminal:${sessionCode}:`.length));
    } else {
      setManualTabs((tabs) => tabs.filter((tab) => tab.key !== key));
    }
    if (remainingTabs?.length === 0) changeBottomPanel?.(false);
  }

  return (
    <>
      <ResizableHandle
        withHandle
        disabled={!panel.panelOpen || maximized}
        className={!panel.panelOpen || maximized ? "hidden" : ""}
        onPointerDown={panel.handlePanelPointerDown}
        onPointerMove={panel.handlePanelPointerMove}
        onPointerUp={panel.handlePanelPointerEnd}
        onPointerCancel={panel.handlePanelPointerEnd}
      />
      <ResizablePanel
        panelRef={panel.panelRef}
        className="border-t border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)]"
        collapsible
        collapsedSize={0}
        defaultSize={0}
        minSize={100}
        maxSize={maximized ? "100%" : "70%"}
        onResize={panel.panelResize}
        data-theme-part="app-bottom-sidebar"
        data-theme-anchor="panel"
      >
        <EditableTabs
          items={items}
          activeKey={activeKey}
          createTab={createManualTab}
          closeTabRequest={closeTabRequest}
          onChange={onChange}
          onEdit={onEdit}
          keepMounted
          preserveInternalTabs={false}
          renderAddTrigger={({ onAdd }) => (
            <>
              <button type="button" aria-label={t("agentTerminal.newTab")} title={t("agentTerminal.newTab")} className="shrink-0 rounded p-1 hover:bg-background" onClick={onAdd}>
                <Plus aria-hidden="true" size={16} />
              </button>
              {sessionCode && runningTerminals.length > 0 ? (
                <DropdownMenu>
                  <DropdownMenuTrigger
                    render={
                      <button type="button" className="flex max-w-48 shrink-0 items-center gap-1 rounded border bg-background px-2 py-1 text-xs hover:bg-accent" aria-label={t("agentTerminal.open")}>
                        <span className="truncate">{t("agentTerminal.open")}</span>
                        <ChevronDown aria-hidden="true" size={14} />
                      </button>
                    }
                  />
                  <DropdownMenuContent align="start" side="top" className="w-max max-w-[min(80vw,32rem)]">
                    {runningTerminals.map((item) => (
                      <DropdownMenuItem key={item.terminalId} onClick={() => openAgentTerminal(sessionCode, item.terminalId)} title={`${item.command} · ${item.cwd}`}>
                        <span className="truncate">{item.command} · {item.cwd}</span>
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : null}
            </>
          )}
          renderTrailingAction={() => (
            <button
              type="button"
              aria-label={t(maximized ? "agentTerminal.restorePanel" : "agentTerminal.maximizePanel")}
              title={t(maximized ? "agentTerminal.restorePanel" : "agentTerminal.maximizePanel")}
              className="shrink-0 rounded p-1 text-muted-foreground hover:bg-background hover:text-foreground"
              onClick={toggleMaximized}
            >
              {maximized ? <Minimize2 aria-hidden="true" size={16} /> : <Maximize2 aria-hidden="true" size={16} />}
            </button>
          )}
        />
      </ResizablePanel>
    </>
  );
}
