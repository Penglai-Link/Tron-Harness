import { useTranslation } from "react-i18next";

import type { GraphNodeLegendItem } from "@/domain/knowledge-graph/model";
import { cn } from "@/shared/lib/utils";

interface GraphLegendProps {
  className?: string;
  items: GraphNodeLegendItem[];
}

export function GraphLegend({ className, items }: GraphLegendProps) {
  const { t } = useTranslation(["pages"]);

  if (items.length === 0) return null;

  return (
    <div
      aria-label={t("pages:knowledgeGraph.legend.aria", { defaultValue: "Graph legend" })}
      className={cn(
        "flex max-w-full flex-wrap items-center gap-3 rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2 shadow-[0_10px_28px_rgba(15,23,42,0.08)]",
        className,
      )}
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
