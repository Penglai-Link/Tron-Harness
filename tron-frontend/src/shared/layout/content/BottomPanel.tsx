import { EditableTab, EditableTabs } from "@/components/editable-tabs";
import { ResizableHandle, ResizablePanel } from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import { memo, useCallback, useMemo, useRef } from "react";
import Terminal from "@/components/terminal";

const BOTTOM_PANEL_COLLAPSE_THRESHOLD = 120;

export default memo(function BottomPanel() {
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
        />
      </ResizablePanel>
    </>
  );
});
