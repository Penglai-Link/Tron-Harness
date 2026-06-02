import type { ReactNode } from "react";

import { cn } from "@/shared/lib/utils";

interface InfoTooltipProps {
  align?: "end" | "start";
  ariaLabel: string;
  children: ReactNode;
}

export function InfoTooltip({
  align = "start",
  ariaLabel,
  children,
}: InfoTooltipProps) {
  return (
    <span className="group relative inline-flex items-center align-middle">
      <button
        aria-label={ariaLabel}
        className="inline-flex size-4 items-center justify-center rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-[10px] font-bold leading-none text-[var(--color-on-surface-variant)] transition-colors hover:border-[var(--color-outline)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2"
        type="button"
      >
        ?
      </button>
      <span
        className={cn(
          "pointer-events-none invisible absolute top-full z-30 mt-2 w-80 max-w-[calc(100vw-2rem)] rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface-strong)] px-3 py-2 text-left text-[12px] font-normal normal-case leading-5 text-[var(--color-on-surface)] opacity-0 shadow-lg transition-opacity group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100",
          align === "end" ? "right-0" : "left-0",
        )}
        role="tooltip"
      >
        <span className="block space-y-2">{children}</span>
      </span>
    </span>
  );
}
