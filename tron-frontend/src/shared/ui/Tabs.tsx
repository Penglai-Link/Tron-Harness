import type { ReactNode } from "react";

import { cn } from "@/shared/lib/utils";

export interface SegmentedTabItem {
  label: string;
  value: string;
}

interface SegmentedTabsProps {
  items: readonly SegmentedTabItem[];
  onValueChange: (value: string) => void;
  value: string;
}

export function SegmentedTabs({ items, onValueChange, value }: SegmentedTabsProps) {
  return (
    <div className="inline-flex rounded-lg bg-[var(--color-surface-container)] p-1" role="tablist">
      {items.map((item) => {
        const selected = item.value === value;
        return (
          <button
            className={cn(
              "h-8 rounded-md px-3 text-[13px] font-medium transition-colors",
              selected ? "bg-[var(--color-elevated-surface-strong)] text-[var(--color-on-surface)] shadow-sm" : "text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]",
            )}
            type="button"
            role="tab"
            aria-selected={selected}
            key={item.value}
            onClick={() => onValueChange(item.value)}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}

interface InspectorPanelProps {
  children: ReactNode;
  title: string;
}

export function InspectorPanel({ children, title }: InspectorPanelProps) {
  return (
    <section className="space-y-4">
      <div>
        <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Inspector</p>
        <h2 className="mt-1 text-xl font-semibold">{title}</h2>
      </div>
      {children}
    </section>
  );
}
