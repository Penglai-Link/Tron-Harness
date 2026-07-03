import type { GraphNodeView } from "@/domain/knowledge-graph/model/KnowledgeGraphModel";

export interface GraphNodeColor {
  border: string;
  color: string;
  soft: string;
}

export interface GraphNodeLegendItem extends GraphNodeColor {
  label: string;
}

const fallbackGraphNodeLabel = "GraphNode";

const graphNodeColorPalette: GraphNodeColor[] = [
  { border: "#bfdbfe", color: "#2563eb", soft: "rgba(37,99,235,0.12)" },
  { border: "#bbf7d0", color: "#15803d", soft: "rgba(21,128,61,0.12)" },
  { border: "#fed7aa", color: "#c2410c", soft: "rgba(194,65,12,0.12)" },
  { border: "#ddd6fe", color: "#7c3aed", soft: "rgba(124,58,237,0.12)" },
  { border: "#fecdd3", color: "#be123c", soft: "rgba(190,18,60,0.12)" },
  { border: "#bae6fd", color: "#0369a1", soft: "rgba(3,105,161,0.12)" },
  { border: "#fde68a", color: "#a16207", soft: "rgba(161,98,7,0.12)" },
  { border: "#99f6e4", color: "#0f766e", soft: "rgba(15,118,110,0.12)" },
  { border: "#fbcfe8", color: "#be185d", soft: "rgba(190,24,93,0.12)" },
  { border: "#c7d2fe", color: "#4338ca", soft: "rgba(67,56,202,0.12)" },
  { border: "#d9f99d", color: "#4d7c0f", soft: "rgba(77,124,15,0.12)" },
  { border: "#e5e7eb", color: "#4b5563", soft: "rgba(75,85,99,0.12)" },
];

function stableHash(value: string) {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

export function graphNodeColorGroup(labels: string[]) {
  return labels.find((label) => label.trim().length > 0)?.trim() ?? fallbackGraphNodeLabel;
}

export function graphNodeColorForGroup(label: string): GraphNodeColor {
  return graphNodeColorPalette[stableHash(label) % graphNodeColorPalette.length];
}

export function graphNodeLegendItems(nodes: GraphNodeView[]): GraphNodeLegendItem[] {
  const counts = new Map<string, number>();
  for (const node of nodes) {
    const label = graphNodeColorGroup(node.labels);
    counts.set(label, (counts.get(label) ?? 0) + 1);
  }

  return Array.from(counts.keys())
    .sort((left, right) => {
      const countDelta = (counts.get(right) ?? 0) - (counts.get(left) ?? 0);
      return countDelta || left.localeCompare(right);
    })
    .map((label) => ({
      label,
      ...graphNodeColorForGroup(label),
    }));
}
