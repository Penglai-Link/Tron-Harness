import type { ReactNode } from "react";
import { CircleHelp } from "lucide-react";

import type { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useTranslation } from "react-i18next";

interface InfoTooltipProps {
  align?: TooltipPrimitive.Positioner.Props["align"];
  ariaLabel: string;
  children: ReactNode;
  variant?: "text" | "icon";
}

export function InfoTooltip({
  align,
  ariaLabel,
  children,
  variant = "icon",
}: InfoTooltipProps) {
  const { t } = useTranslation("common");

  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        delay={500}
        aria-label={ariaLabel}
        className="inline-flex min-h-7 min-w-7 shrink-0 items-center justify-center rounded-sm px-1"
      >
        {variant === "icon" ? <CircleHelp aria-hidden="true" className="size-4" /> : t("actions.help")}
      </TooltipTrigger>
      <TooltipContent align={align}>
        {children}
      </TooltipContent>
    </Tooltip>
  )
}
