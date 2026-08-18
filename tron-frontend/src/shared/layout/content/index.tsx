import { memo, type ReactNode } from "react";

import { ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";

import { Panel } from "./Panel";
import BottomPanel from "./BottomPanel";

interface ContentProps {
  children: ReactNode;
  contentClassName?: string;
  onRightPanelOpenChange?: (open: boolean) => void;
  rightPanel?: ReactNode;
  showBottomPanel?: boolean;
}

export const Content = memo(function Content({
  children,
  contentClassName,
  onRightPanelOpenChange,
  rightPanel,
  showBottomPanel,
}: ContentProps) {
  return (
    <ResizablePanelGroup orientation="vertical" className="h-full w-full">
      <ResizablePanel defaultSize="75%">
        <Panel
          contentClassName={contentClassName}
          onRightPanelOpenChange={onRightPanelOpenChange}
          rightPanel={rightPanel}
        >
          {children}
        </Panel>
      </ResizablePanel>
      {showBottomPanel ? <BottomPanel /> : null}
    </ResizablePanelGroup>
  );
});
