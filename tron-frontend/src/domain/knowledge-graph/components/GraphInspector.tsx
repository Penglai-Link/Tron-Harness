import { Copy, ExternalLink, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  GraphEdgeView,
  GraphNodeView,
  GraphScopeView,
} from "@/domain/knowledge-graph/model";
import { isEmbeddingVectorProperty } from "@/domain/knowledge-graph/model";
import {
  InspectorPanel,
} from "@/shared";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface GraphInspectorProps {
  edges: GraphEdgeView[];
  node: GraphNodeView;
  onClose: () => void;
  onExpandNode: (nodeCode: string) => void;
  scope: GraphScopeView;
}

function formatValue(value: unknown) {
  if (value === null || value === undefined || value === "") return "-";
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

function visiblePropertyRows(properties: GraphNodeView["properties"] | GraphEdgeView["properties"]) {
  return Object.entries(properties).filter(([key]) => !isEmbeddingVectorProperty(key));
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

export function GraphInspector({ edges, node, onClose, onExpandNode, scope }: GraphInspectorProps) {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:knowledgeGraph.${key}`, { defaultValue, ...options })
  );
  const primaryLabel = node.labels[0] ?? "GraphNode";

  return (
    <InspectorPanel title={pageT("inspector.title", "Node Inspector")}>
      <div className="flex items-start justify-between gap-3 border-b border-[var(--color-outline-variant)] pb-4">
        <div className="min-w-0">
          <h2 className="break-words text-xl font-semibold">{node.nodeCode}</h2>
          <div className="mt-2 flex flex-wrap items-center gap-[12px]">
            <Badge variant={primaryLabel === "RagSkill" ? "info" : "outline"}>{primaryLabel}</Badge>
            <Badge variant={node.properties.status === "ACTIVE" ? "success" : "info"}>{formatValue(node.properties.status ?? "ACTIVE")}</Badge>
          </div>
        </div>
        <Button size="icon" variant="ghost" type="button" aria-label={pageT("inspector.closeAria", "Close node inspector")} onClick={onClose}>
          <X size={16} aria-hidden="true" />
        </Button>
      </div>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          {pageT("inspector.sections.scope", "Scope")}
        </h3>
        <KeyValueRows rows={[
          ["namespace", scope.namespace],
          ["namedGraph", scope.namedGraph],
          ["workspaceCode", scope.workspaceCode],
          ["scopeModes", scope.scopeModes.join(", ")],
          ["inheritanceMode", scope.inheritanceMode],
          ["appCode", scope.appCode],
          ["userCode", scope.userCode],
        ]} />
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          {pageT("inspector.sections.nodeFields", "Node Fields")}
        </h3>
        <KeyValueRows rows={[
          ["nodeCode", node.nodeCode],
          ["labels", node.labels.join(", ")],
          ["scopeType", node.scopeType],
          ["originWorkspaceCode", node.originWorkspaceCode],
          ["originUserCode", node.originUserCode],
          ["inherited", node.inherited],
          ["editable", node.editable],
          ["createdAt", node.createdAt.slice(0, 10)],
          ["updatedAt", node.updatedAt.slice(0, 10)],
        ]} />
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          {pageT("inspector.sections.properties", "Properties")}
        </h3>
        <KeyValueRows rows={visiblePropertyRows(node.properties)} />
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          {pageT("inspector.sections.relationships", "Relationships")}
        </h3>
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
                <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">
                  {edge.scopeType} · {edge.originWorkspaceCode}
                  {edge.originUserCode ? ` · ${edge.originUserCode}` : ""}
                  {edge.inherited ? ` · ${pageT("inspector.edgeFlags.inherited", "inherited")}` : ""}
                  {!edge.editable ? ` · ${pageT("inspector.edgeFlags.readOnly", "read-only")}` : ""}
                </p>
                {visiblePropertyRows(edge.properties).length ? (
                  <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">
                    {visiblePropertyRows(edge.properties).map(([key, value]) => `${key}=${formatValue(value)}`).join(" · ")}
                  </p>
                ) : null}
              </div>
            );
          })}
        </div>
      </section>

      <section className="space-y-2">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          {pageT("inspector.sections.traversal", "Traversal")}
        </h3>
        <KeyValueRows rows={[
          ["startNodeCodes", `[${node.nodeCode}]`],
          ["direction", pageT("inspector.traversal.outgoing", "Outgoing")],
          ["maxDepth", 2],
          ["limit", 25],
        ]} />
      </section>

      <div className="grid grid-cols-1 gap-[12px] sm:grid-cols-2 lg:grid-cols-1">
        <Button type="button" variant="secondary" onClick={() => onExpandNode(node.nodeCode)}>
          <ExternalLink size={16} aria-hidden="true" />
          {pageT("inspector.actions.expand", "Expand Node")}
        </Button>
        <Button type="button">
          <ExternalLink size={16} aria-hidden="true" />
          {pageT("inspector.actions.open", "Open Node")}
        </Button>
        <Button type="button" variant="secondary">
          <Copy size={16} aria-hidden="true" />
          {pageT("inspector.actions.copyNodeCode", "Copy node_code")}
        </Button>
      </div>
    </InspectorPanel>
  );
}
