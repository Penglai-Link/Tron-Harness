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
  onAdd: () => void;
  onClose: (key: string) => void;
  className?: string;
}

export function EditableTabList({
  tabs,
  hideAdd,
  renderAddTrigger,
  onAdd,
  onClose,
  className,
}: EditableTabListProps) {
  return (
    <div className={cn("w-full bg-muted", className)}>
      <TabsList className="bg-transparent">
        {tabs.map((tab) => (
          <EditableTabTrigger key={tab.key} onClose={onClose} tab={tab} />
        ))}
        {!hideAdd ? (
          <Button onClick={onAdd} size="icon-sm" type="button" variant="ghost">
            <Plus aria-hidden="true" />
          </Button>
        ) : null}
      </TabsList>
      {renderAddTrigger ? (
        <div className={cn("shrink-0 pb-1")}>{renderAddTrigger({ onAdd })}</div>
      ) : null}
    </div>
  );
}
