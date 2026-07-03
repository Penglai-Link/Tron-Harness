import { Copy, ExternalLink, X } from "lucide-react";

import type {
  GraphEdgeView,
  GraphNodeView,
  GraphScopeView,
} from "@/domain/knowledge-graph/model";
import {
  Badge,
  Button,
  InspectorPanel,
} from "@/shared";

interface GraphInspectorProps {
  edges: GraphEdgeView[];
  node: GraphNodeView;
  onClose: () => void;
  scope: GraphScopeView;
}

function formatValue(value: unknown) {
  if (value === null || value === undefined || value === "") return "-";
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

function KeyValueRows({ rows }: { rows: Array<[string, unknown]> }) {
  return (
    <div className="overflow-hidden rounded-xl border border-[var(--color-outline-variant)]">
      {rows.map(([label, value]) => (
        <div className="flex justify-between gap-3 border-b border-[var(--color-outline-variant)] px-3 py-2 text-sm last:border-0" key={label}>
          <span className="text-[var(--color-on-surface-variant)]">{label}</span>
          <span className="min-w-0 break-words text-right font-medium">{formatValue(value)}</span>
        </div>
      ))}
    </div>
  );
}

export function GraphInspector({ edges, node, onClose, scope }: GraphInspectorProps) {
  const primaryLabel = node.labels[0] ?? "GraphNode";

  return (
    <InspectorPanel title="Node Inspector">
      <div className="flex items-start justify-between gap-3 border-b border-[var(--color-outline-variant)] pb-4">
        <div className="min-w-0">
          <h2 className="break-words text-xl font-semibold">{node.nodeCode}</h2>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <Badge tone={primaryLabel === "RagSkill" ? "info" : "outline"}>{primaryLabel}</Badge>
            <Badge tone={node.properties.status === "ACTIVE" ? "success" : "info"}>{formatValue(node.properties.status ?? "ACTIVE")}</Badge>
          </div>
        </div>
        <Button size="icon" variant="ghost" type="button" aria-label="Close node inspector" onClick={onClose}>
          <X size={16} aria-hidden="true" />
        </Button>
      </div>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Scope</h3>
        <KeyValueRows rows={[
          ["namespace", scope.namespace],
          ["namedGraph", scope.namedGraph],
          ["workspaceCode", scope.workspaceCode],
          ["appCode", scope.appCode],
          ["userCode", scope.userCode],
        ]} />
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Node Fields</h3>
        <KeyValueRows rows={[
          ["nodeCode", node.nodeCode],
          ["labels", node.labels.join(", ")],
          ["createdAt", node.createdAt.slice(0, 10)],
          ["updatedAt", node.updatedAt.slice(0, 10)],
          ["embedding", node.properties.embeddingJson ?? (node.embeddingText ? "indexed" : "not indexed")],
        ]} />
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Properties</h3>
        <KeyValueRows rows={Object.entries(node.properties)} />
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Relationships</h3>
        <div className="space-y-2">
          {edges.map((edge) => {
            const target = edge.sourceNodeCode === node.nodeCode ? edge.targetNodeCode : edge.sourceNodeCode;
            return (
              <div className="rounded-xl border border-[var(--color-outline-variant)] p-3 text-sm" key={edge.edgeCode}>
                <div className="flex items-start justify-between gap-3">
                  <p className="font-mono text-[12px] font-semibold text-[var(--color-accent)]">{edge.relationship}</p>
                  <span className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{edge.edgeCode}</span>
                </div>
                <p className="mt-1 font-medium">{target}</p>
                {Object.keys(edge.properties).length ? (
                  <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">
                    {Object.entries(edge.properties).map(([key, value]) => `${key}=${formatValue(value)}`).join(" · ")}
                  </p>
                ) : null}
              </div>
            );
          })}
        </div>
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Traversal</h3>
        <KeyValueRows rows={[
          ["startNodeCodes", `[${node.nodeCode}]`],
          ["direction", "Outgoing"],
          ["maxDepth", 2],
          ["limit", 25],
        ]} />
      </section>

      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-1">
        <Button type="button">
          <ExternalLink size={16} aria-hidden="true" />
          Open Node
        </Button>
        <Button type="button" variant="secondary">
          <Copy size={16} aria-hidden="true" />
          Copy node_code
        </Button>
      </div>
    </InspectorPanel>
  );
}
