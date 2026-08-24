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
  EditableTabEditInfo,
  OperateType,
} from "@/components/editable-tabs/types";

const BOTTOM_PANEL_COLLAPSE_THRESHOLD = 120;
const BOTTOM_PANEL_DEFAULT_SIZE = "25%";

interface IPanelProps {
  changeBottomPanel?: Dispatch<SetStateAction<boolean>>;
  visible: boolean;
}

export default memo(function BottomPanel({ changeBottomPanel, visible }: IPanelProps) {
  const nextTabIdRef = useRef(1);
  const [hasTerminal, setHasTerminal] = useState(visible);
  const createTab = useCallback((): EditableTab => {
    const tabId = nextTabIdRef.current++;

    return {
      key: `terminal-${tabId}`,
      label: "tron",
      children: <Terminal />,
      closable: true,
    };
  }, []);
  const items = useMemo(() => (hasTerminal ? [createTab()] : []), [createTab, hasTerminal]);
  const { panelRef, panelResize } = usePanel({
    collapseThreshold: BOTTOM_PANEL_COLLAPSE_THRESHOLD,
    initiallyOpen: visible,
    openEventName: null,
    toggleEventName: null,
  });

  useEffect(() => {
    if (visible) {
      panelRef.current?.resize(BOTTOM_PANEL_DEFAULT_SIZE);
      panelRef.current?.expand();
      return;
    }

    panelRef.current?.collapse();
  }, [panelRef, visible]);

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
      <ResizableHandle className={visible ? "w-full" : "hidden"} />
      <ResizablePanel
        panelRef={panelRef}
        collapsible
        collapsedSize={0}
        defaultSize={visible ? BOTTOM_PANEL_DEFAULT_SIZE : 0}
        maxSize="70%"
        minSize={10}
        onResize={panelResize}
      >
        <EditableTabs
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
