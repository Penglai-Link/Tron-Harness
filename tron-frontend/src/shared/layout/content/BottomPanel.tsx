import { useTranslation } from "react-i18next";
import { Dispatch, SetStateAction, useEffect, useState } from "react";
import { ResizableHandle, ResizablePanel } from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import { AgentTerminalPanel } from "@/domain/chat/terminal/AgentTerminalPanel";
import ManualBottomPanel from "./ManualBottomPanel";

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
  const [manual, setManual] = useState(false);
  useEffect(() => {
    setManual(false);
    const open = (event: Event) => {
      if (
        (event as CustomEvent<{ sessionCode: string }>).detail?.sessionCode ===
        sessionCode
      )
        setManual(false);
    };
    window.addEventListener("tron:open-agent-terminal", open);
    return () => window.removeEventListener("tron:open-agent-terminal", open);
  }, [sessionCode]);
  const panel = usePanel({
    collapseThreshold: 120,
    initialSize: "25%",
    open: visible && Boolean(sessionCode) && !manual,
    onOpenChange: (open) => {
      if (sessionCode) changeBottomPanel?.(open);
    },
  });
  return (
    <>
      <ManualBottomPanel
        visible={visible && (!sessionCode || manual)}
        changeBottomPanel={
          sessionCode && !manual ? undefined : changeBottomPanel
        }
        header={
          sessionCode ? (
            <button
              className="border-b px-3 py-1 text-left text-sm"
              onClick={() => setManual(false)}
            >
              {t("agentTerminal.backToAgent")}
            </button>
          ) : null
        }
      />
      <ResizableHandle
        withHandle
        disabled={!panel.panelOpen}
        className={!panel.panelOpen ? "hidden" : ""}
        onPointerDown={panel.handlePanelPointerDown}
        onPointerMove={panel.handlePanelPointerMove}
        onPointerUp={panel.handlePanelPointerEnd}
        onPointerCancel={panel.handlePanelPointerEnd}
      />
      <ResizablePanel
        panelRef={panel.panelRef}
        collapsible
        collapsedSize={0}
        defaultSize={0}
        minSize={100}
        maxSize="70%"
        onResize={panel.panelResize}
      >
        {sessionCode ? (
          <AgentTerminalPanel
            key={sessionCode}
            sessionCode={sessionCode}
            visible={visible && !manual}
            onShowManual={() => setManual(true)}
          />
        ) : null}
      </ResizablePanel>
    </>
  );
}
