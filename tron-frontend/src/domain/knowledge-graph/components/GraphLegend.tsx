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
    <details
      aria-label={t("pages:knowledgeGraph.legend.aria", { defaultValue: "Graph legend" })}
      className={cn(
        "max-w-full rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2 shadow-[0_10px_28px_rgba(15,23,42,0.08)]",
        className,
      )}
    >
      <summary className="cursor-pointer text-xs text-[var(--color-on-surface-variant)]">{t("pages:knowledgeGraph.legend.aria")}</summary>
      <div className="mt-2 flex max-h-32 flex-wrap gap-3 overflow-y-auto">
      {items.map((item) => (
        <span className="inline-flex items-center gap-1.5 text-[12px] text-[var(--color-on-surface-variant)]" key={item.label}>
          {item.label}
        </span>
      ))}
      </div>
    </details>
  );
}
