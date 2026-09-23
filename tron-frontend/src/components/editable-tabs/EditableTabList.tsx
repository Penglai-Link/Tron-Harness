import { Plus } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { TabsList } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

import { EditableTabTrigger } from "./EditableTabTrigger";
import type { EditableTab, EditableTabAddTriggerProps } from "./types";

interface EditableTabListProps {
  tabs: readonly EditableTab[];
  hideAdd: boolean;
  renderAddTrigger?: (props: EditableTabAddTriggerProps) => ReactNode;
  renderTrailingAction?: () => ReactNode;
  onAdd: () => void;
  onClose: (key: string) => void;
  className?: string;
}

export function EditableTabList({
  tabs,
  hideAdd,
  renderAddTrigger,
  renderTrailingAction,
  onAdd,
  onClose,
  className,
}: EditableTabListProps) {
  return (
    <div className={cn("flex w-full min-w-0 items-center bg-muted", className)}>
      <div className="min-w-0 flex-1 overflow-x-auto overflow-y-hidden">
        <TabsList className="bg-transparent gap-1">
          {tabs.filter((tab) => !tab.hidden).map((tab) => (
            <EditableTabTrigger key={tab.key} onClose={onClose} tab={tab} />
          ))}
          {!hideAdd ? (
            renderAddTrigger ? (
              renderAddTrigger({ onAdd })
            ) : (
              <Button aria-label="Add tab" onClick={onAdd} size="icon-sm" type="button" variant="ghost">
                <Plus aria-hidden="true" />
              </Button>
            )
          ) : null}
        </TabsList>
      </div>
      {renderTrailingAction ? <div className="flex shrink-0 items-center px-1">{renderTrailingAction()}</div> : null}
    </div>
  );
}
