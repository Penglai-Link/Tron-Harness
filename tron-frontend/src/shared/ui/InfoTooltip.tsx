import type { ReactNode } from "react";
import { useState } from "react";

import type { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { CircleQuestionMark } from "lucide-react";

interface InfoTooltipProps {
  align?: TooltipPrimitive.Positioner.Props["align"];
  ariaLabel: string;
  children: ReactNode;
}

export function InfoTooltip({
  align,
  ariaLabel,
  children,
}: InfoTooltipProps) {
  const [open, setOpen] = useState(false);
  const title = typeof children === "string" ? children : ariaLabel;

  return (
    <Tooltip open={open} onOpenChange={setOpen}>
      <TooltipTrigger
        type="button"
        delay={0}
        aria-label={ariaLabel}
        title={title}
        onBlur={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
      >
        <CircleQuestionMark size={14} />
      </TooltipTrigger>
      <TooltipContent align={align}>
        {children}
      </TooltipContent>
    </Tooltip>
  )
}
