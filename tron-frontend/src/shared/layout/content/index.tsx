import { Dispatch, memo, SetStateAction, type ReactNode } from "react";

import { ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";

import { Panel } from "./Panel";
import BottomPanel from "./BottomPanel";

interface ContentProps {
  children: ReactNode;
  sessionCode?: string;
  contentClassName?: string;
  showBottomPanel?: boolean;
  changeBottomPanel?: Dispatch<SetStateAction<boolean>>;
}

export const Content = memo(function Content({
  children,
  sessionCode,
  contentClassName,
  showBottomPanel,
  changeBottomPanel
}: ContentProps) {
  return (
    <ResizablePanelGroup orientation="vertical" className="h-full w-full">
      <ResizablePanel defaultSize="75%">
        <Panel
          contentClassName={contentClassName}
        >
          {children}
        </Panel>
      </ResizablePanel>
      <BottomPanel
        sessionCode={sessionCode}
        changeBottomPanel={changeBottomPanel}
        visible={showBottomPanel ?? false}
      />
    </ResizablePanelGroup>
  );
});
