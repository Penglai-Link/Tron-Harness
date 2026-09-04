import { EditableTab, EditableTabs } from "@/components/editable-tabs";
import { ResizableHandle, ResizablePanel } from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import {
  Dispatch,
  memo,
  SetStateAction,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import Terminal from "@/components/terminal";
import {
  type EditableTabCloseRequest,
  type EditableTabEditInfo,
  OperateType,
} from "@/components/editable-tabs/types";

const BOTTOM_PANEL_COLLAPSE_THRESHOLD = 120;
const BOTTOM_PANEL_DEFAULT_SIZE = "25%";

interface IPanelProps {
  changeBottomPanel?: Dispatch<SetStateAction<boolean>>;
  visible: boolean;
}

export default memo(function BottomPanel({
  changeBottomPanel,
  visible,
}: IPanelProps) {
  const nextTabIdRef = useRef(1);
  const nextCloseRequestIdRef = useRef(1);
  const [hasTerminal, setHasTerminal] = useState(visible);
  const [closeTabRequest, setCloseTabRequest] =
    useState<EditableTabCloseRequest | null>(null);
  const requestTabClose = useCallback((key: string) => {
    setCloseTabRequest({ id: nextCloseRequestIdRef.current++, key });
  }, []);
  const createTab = useCallback((): EditableTab => {
    const tabId = nextTabIdRef.current++;
    const key = `terminal-${tabId}`;

    return {
      key,
      label: "tron",
      children: <Terminal onExit={() => requestTabClose(key)} />,
      closable: true,
    };
  }, [requestTabClose]);
  const items = useMemo(
    () => (hasTerminal ? [createTab()] : []),
    [createTab, hasTerminal],
  );
  const {
    handlePanelPointerDown,
    handlePanelPointerEnd,
    handlePanelPointerMove,
    panelOpen,
    panelRef,
    panelResize,
  } = usePanel({
    collapseThreshold: BOTTOM_PANEL_COLLAPSE_THRESHOLD,
    initialSize: BOTTOM_PANEL_DEFAULT_SIZE,
    onOpenChange: (open) => changeBottomPanel?.(open),
    open: visible,
  });

  useEffect(() => {
    if (visible && !hasTerminal) {
      setHasTerminal(true);
    }
  }, [hasTerminal, visible]);

  function onEdit(action: OperateType, { remainingTabs }: EditableTabEditInfo) {
    if (action === OperateType.Remove && remainingTabs?.length === 0) {
      setHasTerminal(false);
      changeBottomPanel?.(false);
    }
  }

  return (
    <>
      <ResizableHandle
        withHandle
        onPointerDown={handlePanelPointerDown}
        onPointerMove={handlePanelPointerMove}
        onPointerUp={handlePanelPointerEnd}
        onPointerCancel={handlePanelPointerEnd}
      />
      <ResizablePanel
        panelRef={panelRef}
        collapsible
        collapsedSize={0}
        defaultSize={panelOpen ? BOTTOM_PANEL_DEFAULT_SIZE : 0}
        maxSize="70%"
        minSize={100}
        onResize={panelResize}
      >
        <EditableTabs
          closeTabRequest={closeTabRequest}
          createTab={createTab}
          defaultActiveKey={items[0]?.key}
          items={items}
          keepMounted
          onEdit={onEdit}
        />
      </ResizablePanel>
    </>
  );
});
