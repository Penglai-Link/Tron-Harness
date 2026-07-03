import { Database } from "lucide-react";

import type { GraphNodeView } from "@/domain/knowledge-graph/model";
import { Badge } from "@/shared";
import { cn } from "@/shared/lib/utils";

interface GraphResultRailProps {
  nodes: GraphNodeView[];
  onSelectNode: (nodeCode: string) => void;
  selectedNodeCode: string;
}

export function GraphResultRail({ nodes, onSelectNode, selectedNodeCode }: GraphResultRailProps) {
  return (
    <aside className="min-h-[620px] border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]">
      <div className="border-b border-[var(--color-outline-variant)] px-4 py-3">
        <h2 className="text-base font-semibold">Query Results</h2>
        <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">{nodes.length} graph nodes</p>
      </div>
      <div className="max-h-[568px] space-y-1 overflow-y-auto p-3">
        {nodes.map((node) => {
          const selected = node.nodeCode === selectedNodeCode;
          const primaryLabel = node.labels[0] ?? "GraphNode";
          return (
            <button
              className={cn(
                "flex w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors",
                selected
                  ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]"
                  : "border-transparent hover:bg-[var(--color-hover-surface)]",
              )}
              key={node.nodeCode}
              type="button"
              aria-label={`${node.nodeCode} ${primaryLabel}`}
              onClick={() => onSelectNode(node.nodeCode)}
            >
              <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-full bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
                <Database size={15} aria-hidden="true" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate font-mono text-[13px] font-semibold">{node.nodeCode}</span>
                <span className="mt-1 flex flex-wrap items-center gap-1.5">
                  <Badge className="h-5 px-2 text-[11px]" tone={primaryLabel === "RagSkill" ? "info" : "outline"}>{primaryLabel}</Badge>
                  {node.score !== undefined ? (
                    <span className="rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 font-mono text-[11px] text-[var(--color-on-surface-variant)]">
                      {node.score.toFixed(2)}
                    </span>
                  ) : null}
                </span>
              </span>
            </button>
          );
        })}
        {nodes.length === 0 ? (
          <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-4 text-sm text-[var(--color-on-surface-variant)]">No graph nodes match the current filters.</p>
        ) : null}
      </div>
    </aside>
  );
}
