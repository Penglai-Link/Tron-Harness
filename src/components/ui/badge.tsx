import type { HTMLAttributes } from "react";

import { cn } from "@/lib/utils";

type BadgeTone = "neutral" | "accent" | "success" | "warning" | "error";

const badgeToneClasses: Record<BadgeTone, string> = {
  neutral: "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)]",
  accent: "bg-[var(--color-accent-container)] text-[var(--color-on-accent-container)]",
  success: "bg-[var(--color-success-container)] text-[var(--color-on-success-container)]",
  warning: "bg-[var(--color-warning-container)] text-[var(--color-on-warning-container)]",
  error: "bg-[var(--color-error-container)] text-[var(--color-on-error-container)]",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: BadgeTone;
}

export function Badge({ className, tone = "neutral", ...props }: BadgeProps) {
  return (
    <span
      className={cn("inline-flex h-6 items-center rounded-full px-2.5 text-[12px] font-medium leading-none", badgeToneClasses[tone], className)}
      {...props}
    />
  );
}
