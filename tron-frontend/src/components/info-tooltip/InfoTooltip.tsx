import type { ReactNode } from "react";

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
  const title = typeof children === "string" ? children : ariaLabel;

  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        delay={500}
        aria-label={ariaLabel}
        title={title}
      >
        <CircleQuestionMark size={14} />
      </TooltipTrigger>
      <TooltipContent align={align}>
        {children}
      </TooltipContent>
    </Tooltip>
  )
}
