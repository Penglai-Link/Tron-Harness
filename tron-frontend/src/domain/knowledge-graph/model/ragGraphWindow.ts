import type { GraphNodeView, GraphEdgeView } from "./KnowledgeGraphModel";
import type { RagGraphNode, RagGraphEdge, RagGraphWindow } from "./RagGraphModel";
function unique<T>(items: T[], key: (item: T) => string): T[] { return Array.from(new Map(items.map((item) => [key(item), item])).values()); }
export function mergeRagGraphWindow(current: RagGraphWindow | null, next: RagGraphWindow): RagGraphWindow {
  if (!current || current.snapshotId !== next.snapshotId) return next;
  const nodes = new Map(current.nodes.map((node) => [node.id, node]));
  for (const node of next.nodes) {
    const old = nodes.get(node.id);
    nodes.set(node.id, old ? { ...node,
      descriptions: unique([...old.descriptions, ...node.descriptions], (text) => `${text.referenceId}:${text.text}`),
      aliases: unique([...old.aliases, ...node.aliases], (text) => `${text.referenceId}:${text.text}`),
      referenceIds: Array.from(new Set([...old.referenceIds, ...node.referenceIds])),
      hops: unique([...old.hops, ...node.hops].sort((a, b) => b.hops - a.hops), (hop) => `${hop.seedId}:${hop.direction}:${hop.nodeId}`),
    } : node);
  }
  const edges = new Map(current.edges.map((edge) => [edge.id, edge]));
  for (const edge of next.edges) {
    const old = edges.get(edge.id);
    edges.set(edge.id, old ? { ...edge, referenceIds: Array.from(new Set([...old.referenceIds, ...edge.referenceIds])) } : edge);
  }
  return { ...next, nodes: Array.from(nodes.values()), edges: Array.from(edges.values()),
    seeds: unique([...current.seeds, ...next.seeds], (seed) => seed.id), references: unique([...current.references, ...next.references], (reference) => reference.id),
    stages: [...current.stages, ...next.stages],
  };
}
// G6's existing display contract also serves business graphs. These fields are
// display-only; RAG inspection uses the original source-bound node and references.
export function ragCanvasNodes(nodes: RagGraphNode[]): GraphNodeView[] {
  return nodes.map((node) => ({ nodeCode: node.id, labels: [node.entityType ?? node.kind], properties: { name: node.name },
    createdAt: "", updatedAt: "", editable: false, inherited: false, scopeType: "SPACE", originWorkspaceCode: "" }));
}
export function ragCanvasEdges(edges: RagGraphEdge[]): GraphEdgeView[] {
  return edges.map((edge) => ({ edgeCode: edge.id, sourceNodeCode: edge.from, targetNodeCode: edge.to, relationship: edge.relationship, properties: {},
    createdAt: "", updatedAt: "", editable: false, inherited: false, scopeType: "SPACE", originWorkspaceCode: "" }));
}
