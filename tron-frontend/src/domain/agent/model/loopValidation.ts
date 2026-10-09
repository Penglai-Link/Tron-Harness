import type { LoopDesign, LoopEdge } from "./LoopModel";
export function visibleEdges(d: LoopDesign): LoopEdge[] {
  return d.kind === "Workflow"
    ? d.nodes.slice(1).map((n, i) => ({ from: d.nodes[i].id, to: n.id }))
    : d.edges;
}
export function validateLoop(d: LoopDesign): string | null {
  if (
    !/^[a-z][a-z0-9_-]{0,63}$/.test(d.loopCode) ||
    !d.name.trim() ||
    d.name.length > 100
  )
    return "identity";
  if (
    !Number.isInteger(d.maxParallelNodes) ||
    d.maxParallelNodes < 1 ||
    d.maxParallelNodes > 16
  )
    return "parallel";
  if (d.kind === "React")
    return d.nodes.length || d.edges.length || d.maxParallelNodes !== 1
      ? "nodes"
      : null;
  const bytes = (value: string) => new TextEncoder().encode(value).length;
  if (
    bytes(JSON.stringify(d)) > 262144 ||
    d.nodes.some((n) => bytes(n.instructions) > 65536)
  )
    return "size";
  if (!d.nodes.length || d.nodes.length > 64 || d.edges.length > 256)
    return "nodes";
  const ids = new Set(d.nodes.map((n) => n.id));
  if (
    d.nodes.some(
      (n) =>
        n.position &&
        (!Number.isFinite(n.position.x) ||
          !Number.isFinite(n.position.y) ||
          Math.abs(n.position.x) > 100000 ||
          Math.abs(n.position.y) > 100000),
    )
  )
    return "position";
  if (
    ids.size !== d.nodes.length ||
    d.nodes.some((n) => !/^[a-z][a-z0-9_-]{0,63}$/.test(n.id))
  )
    return "nodeIds";
  if (
    d.kind === "Workflow" &&
    (d.edges.length ||
      d.maxParallelNodes !== 1 ||
      d.nodes.some((n) => n.kind !== "Model" || n.join !== "Selected"))
  )
    return "nodes";
  const pairs = new Set<string>();
  for (const e of d.edges) {
    const key = `${e.from}→${e.to}`;
    if (!ids.has(e.from) || !ids.has(e.to) || pairs.has(key)) return "edges";
    pairs.add(key);
  }
  for (const n of d.nodes) {
    if (n.kind === "Model") {
      if (n.condition !== null) return "branch";
      continue;
    }
    const c = n.condition;
    if (
      d.kind !== "Dag" ||
      n.instructions ||
      !c ||
      (c.pointer && !c.pointer.startsWith("/")) ||
      c.matchTarget === c.elseTarget ||
      !pairs.has(`${c.sourceNode}→${n.id}`) ||
      !pairs.has(`${n.id}→${c.matchTarget}`) ||
      !pairs.has(`${n.id}→${c.elseTarget}`) ||
      d.edges.filter((e) => e.from === n.id).length !== 2
    )
      return "branch";
  }
  const visited = new Set<string>(),
    visiting = new Set<string>();
  function visit(id: string): boolean {
    if (visiting.has(id)) return false;
    if (visited.has(id)) return true;
    visiting.add(id);
    for (const e of d.edges.filter((e) => e.from === id))
      if (!visit(e.to)) return false;
    visiting.delete(id);
    visited.add(id);
    return true;
  }
  return d.nodes.every((n) => visit(n.id)) ? null : "cycle";
}
export function graphPositions(d: LoopDesign) {
  const edges = visibleEdges(d);
  const levels = new Map(d.nodes.map((n) => [n.id, 0]));
  for (let i = 0; i < d.nodes.length; i++) {
    let changed = false;
    for (const e of edges) {
      if (!levels.has(e.from) || !levels.has(e.to)) continue;
      const next = Math.min(d.nodes.length - 1, (levels.get(e.from) ?? 0) + 1);
      if (next > (levels.get(e.to) ?? 0)) {
        levels.set(e.to, next);
        changed = true;
      }
    }
    if (!changed) break;
  }
  const counts = new Map<number, number>();
  return new Map(
    d.nodes.map((n) => {
      const level = levels.get(n.id) ?? 0;
      const row = counts.get(level) ?? 0;
      counts.set(level, row + 1);
      return [
        n.id,
        d.kind === "Dag" && n.position
          ? n.position
          : { x: level * 210 + 24, y: row * 88 + 28 },
      ];
    }),
  );
}
