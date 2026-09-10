import type { ReactNode } from "react";

import { cn } from "@/shared/lib/utils";

interface DetailFieldProps {
  className?: string;
  label: ReactNode;
  labelClassName?: string;
  value: ReactNode;
  valueClassName?: string;
}

export function DetailField({
  className,
  label,
  labelClassName,
  value,
  valueClassName,
}: DetailFieldProps) {
  return (
    <div className={className}>
      <p className={cn("text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]", labelClassName)}>
        {label}
      </p>
      <div className={cn("mt-1 break-words text-sm font-medium text-[var(--color-on-surface)]", valueClassName)}>
        {value}
      </div>
    </div>
  );
}
