import type { GraphNodeLegendItem } from "@/domain/knowledge-graph/model";

interface GraphLegendProps {
  items: GraphNodeLegendItem[];
}

export function GraphLegend({ items }: GraphLegendProps) {
  if (items.length === 0) return null;

  return (
    <div
      aria-label="Graph legend"
      className="absolute bottom-4 right-4 z-20 flex max-w-[min(42rem,calc(100%-14rem))] flex-wrap items-center gap-3 rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2 shadow-[0_10px_28px_rgba(15,23,42,0.08)]"
    >
      {items.map((item) => (
        <span className="inline-flex items-center gap-1.5 text-[12px] text-[var(--color-on-surface-variant)]" key={item.label}>
          <span className="size-2.5 rounded-full" style={{ backgroundColor: item.color }} />
          {item.label}
        </span>
      ))}
    </div>
  );
}
