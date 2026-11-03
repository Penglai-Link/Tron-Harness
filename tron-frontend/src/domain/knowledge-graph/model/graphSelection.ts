import type { GraphEdgeView, GraphNodeView } from "./KnowledgeGraphModel";

interface GraphSelection {
  active: boolean;
  nodes: Set<string>;
  edges: Set<string>;
}

export function resolveGraphSelection(
  nodes: Pick<GraphNodeView, "nodeCode">[],
  edges: Pick<GraphEdgeView, "edgeCode" | "sourceNodeCode" | "targetNodeCode">[],
  selected: string,
  highlightedNodes: string[],
  highlightedEdges: string[],
): GraphSelection {
  const active = Boolean(selected) && nodes.some(node => node.nodeCode === selected);
  const focus: GraphSelection = {
    active,
    nodes: new Set(active ? [selected] : highlightedNodes),
    edges: new Set(active ? [] : highlightedEdges),
  };
  if (active) {
    for (const edge of edges) {
      if (edge.sourceNodeCode !== selected && edge.targetNodeCode !== selected) continue;
      focus.edges.add(edge.edgeCode);
      focus.nodes.add(edge.sourceNodeCode);
      focus.nodes.add(edge.targetNodeCode);
    }
  }
  return focus;
}

export function graphNodeSelectionStyle(focus: GraphSelection, id: string) {
  const related = focus.nodes.has(id);
  return {
    opacity: focus.active && !related ? 0.16 : related ? 1 : 0.92,
    labelOpacity: focus.active && !related ? 0.22 : 1,
  };
}

export function graphEdgeSelectionStyle(focus: GraphSelection, id: string) {
  return { opacity: focus.edges.has(id) ? 0.82 : focus.active ? 0.07 : 0.38 };
}
