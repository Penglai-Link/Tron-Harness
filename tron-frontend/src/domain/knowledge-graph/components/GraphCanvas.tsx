import { useMemo } from "react";
import {
  Boxes,
  Database,
  FileText,
  GitBranch,
  Network,
  Shapes,
  Tag,
  Wrench,
  type LucideIcon,
} from "lucide-react";

import { GraphLegend } from "@/domain/knowledge-graph/components/GraphLegend";
import type {
  GraphEdgeView,
  GraphNodeView,
} from "@/domain/knowledge-graph/model";
import { cn } from "@/shared/lib/utils";

interface GraphCanvasProps {
  edges: GraphEdgeView[];
  highlightedEdgeCodes: string[];
  highlightedNodeCodes: string[];
  nodes: GraphNodeView[];
  onSelectNode: (nodeCode: string) => void;
  selectedNodeCode: string;
  zoom: number;
}

type NodeKind = "skill" | "tool" | "tag" | "prompt" | "text" | "vector" | "ontology" | "lineage" | "default";

interface PositionedNode extends GraphNodeView {
  cluster: string;
  depth: number;
  kind: NodeKind;
  layoutX: number;
  layoutY: number;
}

const canvasWidth = 860;
const canvasHeight = 620;
const focusPoint = { x: 430, y: 305 };

const nodePalette: Record<NodeKind, { border: string; color: string; soft: string }> = {
  default: { border: "#d3d7d7", color: "#7b7f80", soft: "rgba(123,127,128,0.1)" },
  lineage: { border: "#d3d7d7", color: "#7b7f80", soft: "rgba(123,127,128,0.1)" },
  ontology: { border: "#d9c9ef", color: "#7a5e9a", soft: "rgba(122,94,154,0.1)" },
  prompt: { border: "#c6d6f1", color: "#3e69a7", soft: "rgba(47,95,159,0.1)" },
  skill: { border: "#1c1b1b", color: "#1c1b1b", soft: "rgba(28,27,27,0.08)" },
  tag: { border: "#c9ddcc", color: "#4f6f52", soft: "rgba(79,111,82,0.11)" },
  text: { border: "#d5d8d8", color: "#747878", soft: "rgba(116,120,120,0.11)" },
  tool: { border: "#c6d6f1", color: "#2f5f9f", soft: "rgba(47,95,159,0.11)" },
  vector: { border: "#eadbac", color: "#8a6a1f", soft: "rgba(138,106,31,0.12)" },
};

const nodeIcons: Record<NodeKind, LucideIcon> = {
  default: Network,
  lineage: GitBranch,
  ontology: Shapes,
  prompt: FileText,
  skill: Network,
  tag: Tag,
  text: Database,
  tool: Wrench,
  vector: Boxes,
};

const firstHopAnchors: Record<NodeKind, Array<{ x: number; y: number }>> = {
  default: [{ x: 255, y: 330 }, { x: 640, y: 455 }],
  lineage: [{ x: 235, y: 365 }, { x: 210, y: 455 }],
  ontology: [{ x: 690, y: 205 }, { x: 735, y: 305 }],
  prompt: [{ x: 635, y: 190 }, { x: 690, y: 255 }],
  skill: [{ x: 485, y: 515 }, { x: 365, y: 505 }],
  tag: [{ x: 310, y: 448 }, { x: 250, y: 310 }],
  text: [{ x: 640, y: 405 }, { x: 700, y: 360 }],
  tool: [{ x: 255, y: 238 }, { x: 225, y: 148 }],
  vector: [{ x: 700, y: 485 }, { x: 760, y: 415 }],
};

const remoteAnchors: Record<NodeKind, Array<{ x: number; y: number }>> = {
  default: [{ x: 155, y: 500 }, { x: 745, y: 525 }],
  lineage: [{ x: 120, y: 300 }, { x: 120, y: 430 }],
  ontology: [{ x: 770, y: 150 }, { x: 780, y: 285 }, { x: 770, y: 420 }],
  prompt: [{ x: 720, y: 210 }, { x: 745, y: 300 }],
  skill: [{ x: 500, y: 520 }, { x: 570, y: 470 }],
  tag: [{ x: 185, y: 405 }, { x: 205, y: 275 }, { x: 515, y: 505 }],
  text: [{ x: 695, y: 360 }, { x: 755, y: 325 }],
  tool: [{ x: 205, y: 165 }, { x: 190, y: 310 }],
  vector: [{ x: 755, y: 475 }, { x: 715, y: 535 }],
};

const zoneLabels = [
  { color: "#2f5f9f", label: "Tooling", width: 190, x: 205, y: 88 },
  { color: "#4f6f52", label: "Knowledge tags", width: 220, x: 190, y: 390 },
  { color: "#7a5e9a", label: "Ontology scope", width: 175, x: 650, y: 88 },
  { color: "#747878", label: "Indexed content", width: 235, x: 570, y: 360 },
] as const;

function nodeKind(labels: string[]): NodeKind {
  if (labels.includes("RagSkill")) return "skill";
  if (labels.includes("RagTool")) return "tool";
  if (labels.includes("RagTag")) return "tag";
  if (labels.some((label) => label.includes("Prompt"))) return "prompt";
  if (labels.some((label) => label.includes("Vector"))) return "vector";
  if (labels.some((label) => label.includes("Ontology"))) return "ontology";
  if (labels.some((label) => label.includes("TextUnit") || label.includes("Fulltext") || label.includes("Source"))) return "text";
  if (labels.some((label) => label.includes("TronGraph"))) return "lineage";
  return "default";
}

function nodeColor(labels: string[]) {
  return nodePalette[nodeKind(labels)].color;
}

function findNode<T extends GraphNodeView>(nodes: T[], nodeCode: string) {
  return nodes.find((node) => node.nodeCode === nodeCode);
}

function nodeRadius(node: PositionedNode) {
  if (node.depth === 0) return 34;
  if (node.depth === 1) return 25;
  return 20;
}

function edgeLevel(edge: GraphEdgeView, selectedNodeCode: string, highlightedEdgeCodes: string[]) {
  if (edge.sourceNodeCode === selectedNodeCode || edge.targetNodeCode === selectedNodeCode) return "primary";
  if (highlightedEdgeCodes.includes(edge.edgeCode)) return "path";
  return "context";
}

function edgePath(source: PositionedNode, target: PositionedNode) {
  const dx = target.layoutX - source.layoutX;
  const dy = target.layoutY - source.layoutY;
  const length = Math.hypot(dx, dy) || 1;
  const startX = source.layoutX + (dx / length) * (nodeRadius(source) + 3);
  const startY = source.layoutY + (dy / length) * (nodeRadius(source) + 3);
  const endX = target.layoutX - (dx / length) * (nodeRadius(target) + 8);
  const endY = target.layoutY - (dy / length) * (nodeRadius(target) + 8);
  const normalX = -dy / length;
  const normalY = dx / length;
  const direction = source.layoutX <= target.layoutX ? -1 : 1;
  const bend = Math.min(68, Math.max(28, length / 6)) * direction;
  const controlX = (startX + endX) / 2 + normalX * bend;
  const controlY = (startY + endY) / 2 + normalY * bend;
  return `M ${startX} ${startY} Q ${controlX} ${controlY} ${endX} ${endY}`;
}

function relationshipLabelOffset(relationship: string) {
  if (relationship === "REQUIRES_TOOL") return { x: -16, y: -14 };
  if (relationship === "HAS_TAG") return { x: -14, y: 14 };
  if (relationship === "USES_PROMPT_TEMPLATE") return { x: 14, y: -16 };
  if (relationship === "LINKS") return { x: 16, y: 12 };
  return { x: 0, y: 0 };
}

function edgeLabelPoint(source: PositionedNode, target: PositionedNode, edge: GraphEdgeView, level: ReturnType<typeof edgeLevel>) {
  const dx = target.layoutX - source.layoutX;
  const dy = target.layoutY - source.layoutY;
  const length = Math.hypot(dx, dy) || 1;
  const normalX = -dy / length;
  const normalY = dx / length;
  const direction = source.layoutX <= target.layoutX ? -1 : 1;
  const bend = Math.min(58, Math.max(20, length / 8)) * direction;
  const relationOffset = relationshipLabelOffset(edge.relationship);

  if (level === "primary" && (source.depth === 0 || target.depth === 0)) {
    const focus = source.depth === 0 ? source : target;
    const neighbor = source.depth === 0 ? target : source;
    return {
      x: focus.layoutX + (neighbor.layoutX - focus.layoutX) * 0.62 + normalX * 12 + relationOffset.x,
      y: focus.layoutY + (neighbor.layoutY - focus.layoutY) * 0.62 + normalY * 12 + relationOffset.y,
    };
  }

  return {
    x: (source.layoutX + target.layoutX) / 2 + normalX * bend + relationOffset.x,
    y: (source.layoutY + target.layoutY) / 2 + normalY * bend - 10 + relationOffset.y,
  };
}

function assignAnchor(kind: NodeKind, index: number, firstHop: boolean) {
  const anchors = firstHop ? firstHopAnchors[kind] : remoteAnchors[kind];
  const anchor = anchors[index % anchors.length];
  const round = Math.floor(index / anchors.length);
  const offset = firstHop ? 34 : 42;
  const direction = index % 2 === 0 ? 1 : -1;
  return {
    x: anchor.x + round * 28 * direction,
    y: anchor.y + round * offset,
  };
}

function buildSemanticLayout(nodes: GraphNodeView[], edges: GraphEdgeView[], selectedNodeCode: string) {
  const selectedNode = findNode(nodes, selectedNodeCode) ?? nodes[0];
  const selectedCode = selectedNode?.nodeCode ?? "";
  const firstHopCodes = new Set<string>();

  for (const edge of edges) {
    if (edge.sourceNodeCode === selectedCode) firstHopCodes.add(edge.targetNodeCode);
    if (edge.targetNodeCode === selectedCode) firstHopCodes.add(edge.sourceNodeCode);
  }

  const firstHopCounts = new Map<NodeKind, number>();
  const remoteCounts = new Map<NodeKind, number>();
  const sortedNodes = [...nodes].sort((left, right) => {
    if (left.nodeCode === selectedCode) return -1;
    if (right.nodeCode === selectedCode) return 1;
    const leftFirst = firstHopCodes.has(left.nodeCode);
    const rightFirst = firstHopCodes.has(right.nodeCode);
    if (leftFirst !== rightFirst) return leftFirst ? -1 : 1;
    return left.nodeCode.localeCompare(right.nodeCode);
  });

  return sortedNodes.map<PositionedNode>((node) => {
    const kind = nodeKind(node.labels);
    if (node.nodeCode === selectedCode) {
      return {
        ...node,
        cluster: "Focus",
        depth: 0,
        kind,
        layoutX: focusPoint.x,
        layoutY: focusPoint.y,
      };
    }

    const firstHop = firstHopCodes.has(node.nodeCode);
    const counts = firstHop ? firstHopCounts : remoteCounts;
    const index = counts.get(kind) ?? 0;
    counts.set(kind, index + 1);
    const anchor = assignAnchor(kind, index, firstHop);

    return {
      ...node,
      cluster: firstHop ? "Primary relationships" : "Context",
      depth: firstHop ? 1 : 2,
      kind,
      layoutX: anchor.x,
      layoutY: anchor.y,
    };
  });
}

function edgeStroke(level: ReturnType<typeof edgeLevel>) {
  if (level === "primary") return "#2f5f9f";
  if (level === "path") return "#7a5e9a";
  return "#cfd4d4";
}

export function GraphCanvas({
  edges,
  highlightedEdgeCodes,
  highlightedNodeCodes,
  nodes,
  onSelectNode,
  selectedNodeCode,
  zoom,
}: GraphCanvasProps) {
  const positionedNodes = useMemo(
    () => buildSemanticLayout(nodes, edges, selectedNodeCode),
    [edges, nodes, selectedNodeCode],
  );
  const nodeByCode = useMemo(
    () => new Map(positionedNodes.map((node) => [node.nodeCode, node])),
    [positionedNodes],
  );
  const visibleNodeCodes = new Set(positionedNodes.map((node) => node.nodeCode));
  const renderEdges = edges.filter((edge) => visibleNodeCodes.has(edge.sourceNodeCode) && visibleNodeCodes.has(edge.targetNodeCode));
  const scale = zoom / 100;

  return (
    <section
      className="relative min-h-[620px] min-w-0 overflow-hidden bg-[var(--color-surface-container-lowest)]"
      aria-label="Semantic focus graph canvas"
      style={{
        backgroundImage: "radial-gradient(circle at 1px 1px, rgba(68,71,72,0.14) 1px, transparent 0), linear-gradient(135deg, rgba(255,255,255,0.72), rgba(243,244,244,0.52))",
        backgroundSize: "20px 20px, 100% 100%",
      }}
    >
      <div
        className="relative origin-top-left transition-transform"
        style={{
          height: canvasHeight,
          transform: `scale(${scale})`,
          width: canvasWidth,
        }}
      >
        <div className="absolute left-5 top-5 z-20 rounded-full border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)] shadow-sm">
          Primary relationships
        </div>
        <svg
          className="absolute inset-0"
          height={canvasHeight}
          viewBox={`0 0 ${canvasWidth} ${canvasHeight}`}
          width={canvasWidth}
          aria-hidden="true"
        >
          <defs>
            <marker id="graph-arrow" markerHeight="8" markerWidth="8" orient="auto" refX="7" refY="4">
              <path d="M 0 0 L 8 4 L 0 8 z" fill="#aeb5b5" />
            </marker>
            <marker id="graph-arrow-active" markerHeight="9" markerWidth="9" orient="auto" refX="8" refY="4.5">
              <path d="M 0 0 L 9 4.5 L 0 9 z" fill="#2f5f9f" />
            </marker>
            <filter id="graph-soft-shadow" height="150%" width="150%" x="-25%" y="-25%">
              <feDropShadow dx="0" dy="8" floodColor="#1f2933" floodOpacity="0.12" stdDeviation="8" />
            </filter>
          </defs>

          {zoneLabels.map((zone) => (
            <g key={zone.label} opacity="0.72">
              <rect
                fill="transparent"
                height="148"
                rx="18"
                stroke={zone.color}
                strokeDasharray="3 8"
                strokeOpacity="0.16"
                width={zone.width}
                x={zone.x}
                y={zone.y}
              />
              <text
                fill={zone.color}
                fontSize="11"
                fontWeight="700"
                letterSpacing="0.4"
                x={zone.x + 14}
                y={zone.y + 24}
              >
                {zone.label}
              </text>
            </g>
          ))}

          {renderEdges.map((edge) => {
            const source = nodeByCode.get(edge.sourceNodeCode);
            const target = nodeByCode.get(edge.targetNodeCode);
            if (!source || !target) return null;
            const level = edgeLevel(edge, selectedNodeCode, highlightedEdgeCodes);
            const active = level !== "context";
            const labelPoint = edgeLabelPoint(source, target, edge, level);
            return (
              <g key={edge.edgeCode} opacity={active ? 1 : 0.58}>
                <path
                  d={edgePath(source, target)}
                  fill="none"
                  markerEnd={active ? "url(#graph-arrow-active)" : "url(#graph-arrow)"}
                  stroke={edgeStroke(level)}
                  strokeDasharray={active ? undefined : "6 8"}
                  strokeLinecap="round"
                  strokeWidth={level === "primary" ? 3 : level === "path" ? 2.2 : 1.4}
                />
                {active ? (
                  <foreignObject height="30" width="178" x={labelPoint.x - 89} y={labelPoint.y - 15}>
                    <div className="flex justify-center">
                      <span className="max-w-[170px] truncate rounded-full border border-[#c6d6f1] bg-white/95 px-2.5 py-1 font-mono text-[10px] font-semibold text-[#2f5f9f] shadow-[0_6px_16px_rgba(47,95,159,0.12)]">
                        {edge.relationship}
                      </span>
                    </div>
                  </foreignObject>
                ) : null}
              </g>
            );
          })}
        </svg>

        {positionedNodes.map((node) => {
          const selected = node.nodeCode === selectedNodeCode;
          const highlighted = highlightedNodeCodes.includes(node.nodeCode) || node.depth <= 1;
          const primaryLabel = node.labels[0] ?? "GraphNode";
          const palette = nodePalette[node.kind];
          const Icon = nodeIcons[node.kind];
          return (
            <button
              className="absolute z-10 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center outline-none"
              key={node.nodeCode}
              type="button"
              aria-label={selected ? `Selected ${node.nodeCode} graph node` : `Select ${node.nodeCode} graph node`}
              style={{ left: node.layoutX, top: node.layoutY }}
              onClick={() => onSelectNode(node.nodeCode)}
            >
              <span
                className={cn(
                  "relative flex items-center justify-center rounded-full border bg-white shadow-[0_9px_24px_rgba(15,23,42,0.12)] transition-[box-shadow,transform]",
                  selected ? "size-[70px] text-white ring-[10px] ring-[#2f5f9f]/10" : node.depth === 1 ? "size-[52px] ring-4 ring-white/80" : "size-11 opacity-90",
                  highlighted && !selected ? "ring-4 ring-[#2f5f9f]/12" : undefined,
                )}
                style={{
                  backgroundColor: selected ? "var(--color-primary)" : "#ffffff",
                  borderColor: selected ? "var(--color-primary)" : palette.border,
                  color: selected ? "var(--color-on-primary)" : palette.color,
                }}
              >
                {selected ? (
                  <span className="absolute inset-[-9px] rounded-full border border-[#2f5f9f]/25" />
                ) : null}
                <Icon size={selected ? 28 : node.depth === 1 ? 20 : 17} strokeWidth={selected ? 2.4 : 2.1} aria-hidden="true" />
              </span>
              <span
                className={cn(
                  "mt-2 max-w-[156px] truncate rounded-md border bg-white/95 px-2 py-1 text-center font-mono shadow-[0_6px_16px_rgba(15,23,42,0.08)]",
                  selected ? "border-[var(--color-primary)] text-[12px] font-bold text-[var(--color-on-surface)]" : "border-[var(--color-outline-variant)] text-[10px] font-semibold text-[var(--color-on-surface-variant)]",
                )}
                title={`${node.nodeCode} / ${primaryLabel}`}
              >
                {node.nodeCode}
              </span>
              {selected ? (
                <span className="mt-1 rounded-full px-2 py-0.5 text-[10px] font-semibold" style={{ backgroundColor: palette.soft, color: palette.color }}>
                  {primaryLabel}
                </span>
              ) : null}
            </button>
          );
        })}
      </div>

      <div className="absolute bottom-4 left-4 z-20 rounded-xl border border-[var(--color-card-border)] bg-white/88 p-2 shadow-[0_10px_28px_rgba(15,23,42,0.08)] backdrop-blur-sm">
        <div className="relative h-28 w-40 overflow-hidden rounded-lg bg-[var(--color-surface-container-low)]">
          {positionedNodes.slice(0, 18).map((node) => (
            <span
              className={cn("absolute rounded-full", node.depth === 0 ? "size-2.5" : "size-1.5")}
              key={`mini-${node.nodeCode}`}
              style={{
                backgroundColor: nodeColor(node.labels),
                left: `${Math.min(95, Math.max(4, (node.layoutX / canvasWidth) * 100))}%`,
                top: `${Math.min(92, Math.max(5, (node.layoutY / canvasHeight) * 100))}%`,
              }}
            />
          ))}
          <span className="absolute left-[28%] top-[22%] h-[56%] w-[48%] rounded-md border border-[var(--color-accent)]/50" />
        </div>
      </div>

      <GraphLegend />
    </section>
  );
}
