import type { LoopDesign, LoopNode } from "./LoopModel";
import { graphPositions } from "./loopValidation";

export type LoopOutlet = "output" | "matched" | "otherwise";
export type CanvasPosition = { x: number; y: number };
export class LoopConnectionError extends Error {
  constructor(public readonly reason: "cycle" | "branch" | "edges") {
    super(reason);
  }
}

export function withCanvasPositions(design: LoopDesign): LoopDesign {
  const positions = graphPositions(design);
  return {
    ...design,
    nodes: design.nodes.map((node) => ({
      ...node,
      position: node.position ?? positions.get(node.id) ?? { x: 24, y: 28 },
    })),
  };
}

export function disconnectLoopEdge(
  design: LoopDesign,
  index: number,
): LoopDesign {
  const edge = design.edges[index];
  if (!edge) return design;
  return {
    ...withCanvasPositions(design),
    edges: design.edges.filter((_, i) => i !== index),
    nodes: withCanvasPositions(design).nodes.map((node) => {
      if (!node.condition) return node;
      const condition = { ...node.condition };
      if (node.id === edge.to && condition.sourceNode === edge.from)
        condition.sourceNode = "";
      if (node.id === edge.from) {
        if (condition.matchTarget === edge.to) condition.matchTarget = "";
        if (condition.elseTarget === edge.to) condition.elseTarget = "";
      }
      return { ...node, condition };
    }),
  };
}

export function connectLoopNodes(
  design: LoopDesign,
  fromIndex: number,
  toIndex: number,
  outlet: LoopOutlet,
): LoopDesign {
  const from = design.nodes[fromIndex],
    to = design.nodes[toIndex];
  if (!from || !to || !from.id || !to.id || from.id === to.id)
    throw new LoopConnectionError("cycle");
  if (new Set(design.nodes.map((node) => node.id)).size !== design.nodes.length)
    throw new LoopConnectionError("edges");
  let next = withCanvasPositions(design);
  if (from.kind === "Branch") {
    if (!from.condition || outlet === "output")
      throw new LoopConnectionError("branch");
    const key = outlet === "matched" ? "matchTarget" : "elseTarget";
    const other = outlet === "matched" ? "elseTarget" : "matchTarget";
    if (from.condition[other] === to.id)
      throw new LoopConnectionError("branch");
    const previous = next.edges.findIndex(
      (e) => e.from === from.id && e.to === from.condition![key],
    );
    if (previous >= 0) next = disconnectLoopEdge(next, previous);
    next.nodes = next.nodes.map((node, index) =>
      index === fromIndex
        ? {
            ...node,
            condition: { ...node.condition!, [key]: to.id },
          }
        : node,
    );
  }
  if (to.kind === "Branch") {
    const previous = next.edges.findIndex(
      (e) => e.to === to.id && e.from === to.condition?.sourceNode,
    );
    if (previous >= 0) next = disconnectLoopEdge(next, previous);
    next.nodes = next.nodes.map((node, index) =>
      index === toIndex
        ? {
            ...node,
            condition: { ...node.condition!, sourceNode: from.id },
          }
        : node,
    );
  }
  if (!next.edges.some((e) => e.from === from.id && e.to === to.id))
    next.edges = [...next.edges, { from: from.id, to: to.id }];
  if (next.edges.length > 256) throw new LoopConnectionError("edges");
  const visited = new Set<string>();
  const reachesSource = (id: string): boolean => {
    if (id === from.id) return true;
    if (visited.has(id)) return false;
    visited.add(id);
    return next.edges
      .filter((edge) => edge.from === id)
      .some((edge) => reachesSource(edge.to));
  };
  if (reachesSource(to.id)) throw new LoopConnectionError("cycle");
  return next;
}

export function clearNodeReferences(
  nodes: LoopNode[],
  removedId: string,
): LoopNode[] {
  return nodes.map((node) =>
    !node.condition
      ? node
      : {
          ...node,
          condition: {
            ...node.condition,
            sourceNode:
              node.condition.sourceNode === removedId
                ? ""
                : node.condition.sourceNode,
            matchTarget:
              node.condition.matchTarget === removedId
                ? ""
                : node.condition.matchTarget,
            elseTarget:
              node.condition.elseTarget === removedId
                ? ""
                : node.condition.elseTarget,
          },
        },
  );
}
