import { EditableTab, EditableTabs } from "@/components/editable-tabs";
import { ResizableHandle, ResizablePanel } from "@/components/ui/resizable";
import { usePanel } from "@/shared/hooks/usePanel";
import { memo } from "react";
import Terminal from "@/components/terminal";

const BOTTOM_PANEL_COLLAPSE_THRESHOLD = 120;

export default memo(function BottomPanel() {
  const { panelRef, panelResize } = usePanel({
    collapseThreshold: BOTTOM_PANEL_COLLAPSE_THRESHOLD,
    initiallyOpen: true,
    openEventName: null,
    toggleEventName: null,
  });

  const items: EditableTab[] = [
    {
      key: `${new Date()}`,
      label: "tron",
      children: <Terminal />,
      closable: true,
    },
  ];

  function createTab() {
    return {
      key: `${new Date()}`,
      label: "tron",
      children: <Terminal />,
      closable: true,
    };
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
          defaultActiveKey="1"
          items={items}
        />
      </ResizablePanel>
    </>
  );
});
