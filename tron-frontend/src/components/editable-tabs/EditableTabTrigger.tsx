import type { MouseEvent } from "react";

import { TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

import type { EditableTab } from "./types";
import { X } from "lucide-react";
import { Button } from "../ui/button";

interface EditableTabTriggerProps {
  tab: EditableTab;
  onClose: (key: string) => void;
}

export function EditableTabTrigger({ tab, onClose }: EditableTabTriggerProps) {
  const handleClose = (event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    onClose(tab.key);
  };

  return (
    <div
      className={cn(
        "flex h-full max-w-[200px] items-center rounded-md p-1 has-data-active:bg-background has-data-active:text-foreground has-data-active:shadow-sm dark:has-data-active:bg-input/30 dark:has-data-active:text-foreground",
      )}
    >
      <TabsTrigger
        value={tab.key}
        className="bg-transparent! shadow-none! data-active:bg-transparent! data-active:text-inherit! dark:data-active:border-transparent! dark:data-active:bg-transparent! dark:data-active:text-inherit!"
      >
        <span className="max-w-40 truncate">{tab.label}</span>
      </TabsTrigger>
      {tab.closable ? (
        <Button
          size="icon-sm"
          variant="ghost"
          className=""
          onClick={handleClose}
        >
          <X size={12} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}
