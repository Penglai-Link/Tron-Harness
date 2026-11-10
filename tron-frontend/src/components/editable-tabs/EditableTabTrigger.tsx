import { ExternalLink, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useRef, type MouseEvent } from "react";

import { TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

import type { EditableTab, EditableTabsProps } from "./types";

import { Button } from "../ui/button";

interface EditableTabTriggerProps {
  tab: EditableTab;
  onClose: (key: string) => void;
  onDetach?: EditableTabsProps["onDetach"];
}

export function EditableTabTrigger({ tab, onClose, onDetach }: EditableTabTriggerProps) {
  const drag = useRef<{ x: number; y: number; bounds: DOMRect } | null>(null);
  const suppressClick = useRef(false);
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
        style={tab.detachable ? { touchAction: "none" } : undefined}
        onPointerDown={(event) => {
          suppressClick.current = false;
          if (!tab.detachable || !onDetach || event.button !== 0) return;
          const bounds = event.currentTarget.closest("[data-tab-strip]")?.getBoundingClientRect();
          if (!bounds) return;
          drag.current = { x: event.clientX, y: event.clientY, bounds };
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerCancel={() => { drag.current = null; }}
        onLostPointerCapture={() => { drag.current = null; }}
        onPointerUp={(event) => {
          const start = drag.current;
          drag.current = null;
          if (!start) return;
          const { bounds } = start;
          const outside = event.clientX < bounds.left - 24 || event.clientX > bounds.right + 24
            || event.clientY < bounds.top - 24 || event.clientY > bounds.bottom + 24;
          if (outside && Math.hypot(event.clientX - start.x, event.clientY - start.y) > 12) {
            suppressClick.current = true;
            onDetach?.(tab, { screenX: event.screenX, screenY: event.screenY });
          }
        }}
        onClickCapture={(event) => {
          if (suppressClick.current) {
            event.preventDefault();
            event.stopPropagation();
            suppressClick.current = false;
          }
        }}
        className="flex-auto w-1/2 bg-transparent! shadow-none! data-active:bg-transparent! data-active:text-inherit! dark:data-active:border-transparent! dark:data-active:bg-transparent! dark:data-active:text-inherit!"
      >
        <span
          className={cn("max-w-40 truncate", tab.labelClassName)}
          title={typeof tab.label === "string" ? tab.label : ""}
        >
          {tab.label}
        </span>
      </TabsTrigger>
      {tab.detachable && onDetach ? (
        <Button aria-label={t("content.detachTab")} title={t("content.detachTab")}
          size="icon-sm" variant="ghost" className="shrink-0"
          onClick={(event) => { event.stopPropagation(); onDetach(tab); }}>
          <ExternalLink size={12} aria-hidden="true" />
        </Button>
      ) : null}
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
