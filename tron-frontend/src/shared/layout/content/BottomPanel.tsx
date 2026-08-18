import { EditableTab, EditableTabs } from "@/components/editable-tabs";
import { ResizableHandle, ResizablePanel } from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import {
  Dispatch,
  memo,
  SetStateAction,
  useCallback,
  useMemo,
  useRef,
} from "react";
import Terminal from "@/components/terminal";
import {
  EditableTabEditInfo,
  OperateType,
} from "@/components/editable-tabs/types";

const BOTTOM_PANEL_COLLAPSE_THRESHOLD = 120;

interface IPanelProps {
  changeBottomPanel?: Dispatch<SetStateAction<boolean>>;
}

export default memo(function BottomPanel({ changeBottomPanel }: IPanelProps) {
  const nextTabIdRef = useRef(1);
  const createTab = useCallback((): EditableTab => {
    const tabId = nextTabIdRef.current++;

    return {
      key: `terminal-${tabId}`,
      label: "tron",
      children: <Terminal />,
      closable: true,
    };
  }, []);
  const items = useMemo(() => [createTab()], [createTab]);
  const { panelRef, panelResize } = usePanel({
    collapseThreshold: BOTTOM_PANEL_COLLAPSE_THRESHOLD,
    initiallyOpen: true,
    openEventName: null,
    toggleEventName: null,
  });

  function onEdit(action: OperateType, { remainingTabs }: EditableTabEditInfo) {
    if (action === OperateType.Remove && remainingTabs?.length === 0) {
      changeBottomPanel?.(false);
    }
  }

  return (
    <>
      <ResizableHandle className="w-full" />
      <ResizablePanel
        panelRef={panelRef}
        collapsible
        collapsedSize={0}
        defaultSize="25%"
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
