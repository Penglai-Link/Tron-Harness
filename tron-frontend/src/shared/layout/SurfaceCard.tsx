import type {
  HTMLAttributes,
  ReactNode,
} from "react";

import { cn } from "@/shared/lib/utils";

export function SurfaceCard({ className, ...props }: HTMLAttributes<HTMLElement>) {
  return <section className={cn("min-w-0 rounded-[8px] border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.04)]", className)} {...props} />;
}

interface MetricCardProps {
  className?: string;
  label: string;
  value: string;
  trend?: string;
  icon?: ReactNode;
}

export function MetricCard({ className, icon, label, trend, value }: MetricCardProps) {
  const hasTrend = Boolean(trend);

  return (
    <div
      className={cn(
        "rounded-[8px] border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-4",
        hasTrend ? "min-h-[132px]" : "min-h-[104px]",
        className,
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
          <p className={cn("font-mono text-2xl font-semibold leading-none tracking-normal", hasTrend ? "mt-2" : "mt-3")}>{value}</p>
        </div>
        {icon ? <div className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">{icon}</div> : null}
      </div>
      {hasTrend ? <p className="mt-3 text-[13px] text-[var(--color-on-surface-variant)]">{trend}</p> : null}
    </div>
  );
}
