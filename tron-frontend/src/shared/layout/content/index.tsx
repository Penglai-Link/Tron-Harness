import { Dispatch, memo, SetStateAction, type ReactNode } from "react";

import { ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";

import { Panel } from "./Panel";
import BottomPanel from "./BottomPanel";

interface ContentProps {
  children: ReactNode;
  contentClassName?: string;
  rightPanel?: ReactNode;
  showBottomPanel?: boolean;
  changeBottomPanel?: Dispatch<SetStateAction<boolean>>;
}

export const Content = memo(function Content({
  children,
  contentClassName,
  rightPanel,
  showBottomPanel,
  changeBottomPanel
}: ContentProps) {
  return (
    <ResizablePanelGroup orientation="vertical" className="h-full w-full">
      <ResizablePanel defaultSize="75%">
        <Panel
          contentClassName={contentClassName}
          rightPanel={rightPanel}
        >
          {children}
        </Panel>
      </ResizablePanel>
      <BottomPanel
        changeBottomPanel={changeBottomPanel}
        visible={showBottomPanel ?? false}
      />
    </ResizablePanelGroup>
  );
});
