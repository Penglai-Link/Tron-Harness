import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { MouseEvent } from "react";

import { TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

import type { EditableTab } from "./types";

import { Button } from "../ui/button";

interface EditableTabTriggerProps {
  tab: EditableTab;
  onClose: (key: string) => void;
}

export function EditableTabTrigger({ tab, onClose }: EditableTabTriggerProps) {
  const { t } = useTranslation("common");
  const handleClose = (event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    onClose(tab.key);
  };

  return (
    <div
      className={cn(
        "flex h-full max-w-[200px] items-center rounded-md p-1 has-data-active:bg-background has-data-active:text-foreground has-data-active:shadow-sm dark:has-data-active:bg-input/30 dark:has-data-active:text-foreground shadow-none!",
        "hover:text-foreground hover:bg-background/50"
      )}
    >
      <TabsTrigger
        value={tab.key}
        className="flex-auto w-1/2 bg-transparent! shadow-none! data-active:bg-transparent! data-active:text-inherit! dark:data-active:border-transparent! dark:data-active:bg-transparent! dark:data-active:text-inherit!"
      >
        <span
          className={cn("max-w-40 truncate", tab.labelClassName)}
          title={typeof tab.label === "string" ? tab.label : ""}
        >
          {tab.label}
        </span>
      </TabsTrigger>
      {tab.closable !== false ? (
        <Button
          aria-label={t("actions.closeTab", "Close tab")}
          size="icon-sm"
          variant="ghost"
          className="flex-shrink-0"
          onClick={handleClose}
        >
          <X size={12} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}
