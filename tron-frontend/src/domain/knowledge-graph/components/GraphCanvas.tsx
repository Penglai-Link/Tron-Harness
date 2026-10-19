import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import {
  CanvasEvent,
  Graph,
  GraphEvent,
  NodeEvent,
  type GraphData,
  type IElementEvent,
  type Point,
} from "@antv/g6";
import { useTranslation } from "react-i18next";

import { GraphLegend } from "@/domain/knowledge-graph/components/GraphLegend";
import type {
  GraphEdgeView,
  GraphLayoutMode,
  GraphMetrics,
  GraphNodeView,
} from "@/domain/knowledge-graph/model";
import {
  graphNodeColorForGroup,
  graphNodeColorGroup,
  graphNodeLegendItems,
  resolveGraphNodeLabel,
} from "@/domain/knowledge-graph/model";
import { cn } from "@/shared/lib/utils";
import { useTheme } from "@/shared/theme/ThemeProvider";

interface GraphCanvasProps {
  edges: GraphEdgeView[];
  highlightedEdgeCodes: string[];
  highlightedNodeCodes: string[];
  layoutMode: GraphLayoutMode;
  metrics: GraphMetrics;
  nodeLabelProperty: string;
  nodes: GraphNodeView[];
  onCameraZoomChange: (zoom: number) => void;
  onClearSelection: () => void;
  onSelectNode: (nodeCode: string) => void;
  selectedNodeCode: string;
  zoom: number;
}

type NodeKind = "skill" | "tool" | "tag" | "prompt" | "text" | "vector" | "ontology" | "lineage" | "code" | "default";

interface PositionedNode extends GraphNodeView {
  colorGroup: string;
  degree: number;
  layoutX: number;
  layoutY: number;
}

interface ViewportPoint {
  x: number;
  y: number;
}

interface GraphBounds {
  height: number;
  maxX: number;
  maxY: number;
  minX: number;
  minY: number;
  width: number;
}

interface MinimapViewport {
  height: number;
  left: number;
  top: number;
  visibleHeight: number;
  visibleWidth: number;
  width: number;
}

interface MinimapDragState {
  offsetXRatio: number;
  offsetYRatio: number;
  pointerId: number;
}

interface EdgeLike {
  edgeCode: string;
  sourceNodeCode: string;
  targetNodeCode: string;
}

interface GraphZoomStyleData {
  edges: NonNullable<GraphData["edges"]>;
  nodes: NonNullable<GraphData["nodes"]>;
}

interface GraphCanvasTheme {
  activeEdgeColor: string;
  canvasBackgroundImage: string;
  edgeLabelBackgroundFill: string;
  edgeLabelBackgroundOpacity: number;
  edgeLabelBackgroundStroke: string;
  edgeStroke: string;
  highlightedNodeStroke: string;
  nodeLabelFill: string;
  selectedNodeStroke: string;
}

type ResolvedGraphLayoutMode = Exclude<GraphLayoutMode, "Smart">;

const fallbackMinimapViewport: MinimapViewport = {
  height: 58,
  left: 26,
  top: 18,
  visibleHeight: 58,
  visibleWidth: 52,
  width: 52,
};

function readGraphCanvasTheme(): GraphCanvasTheme {
  const styles = getComputedStyle(document.documentElement);
  const color = (token: string, semantic: string) => styles.getPropertyValue(token).trim() || styles.getPropertyValue(semantic).trim();
  return {
    activeEdgeColor: color("--tron-graph-active-edge", "--color-accent"),
    canvasBackgroundImage: "radial-gradient(circle at 1px 1px, var(--tron-graph-grid, var(--color-outline-variant)) 1px, transparent 0)",
    edgeLabelBackgroundFill: color("--tron-graph-label-bg", "--color-elevated-surface"),
    edgeLabelBackgroundOpacity: 0.94,
    edgeLabelBackgroundStroke: color("--tron-graph-label-border", "--color-outline-variant"),
    edgeStroke: color("--tron-graph-edge", "--color-outline-variant"),
    highlightedNodeStroke: color("--tron-graph-highlight", "--color-accent"),
    nodeLabelFill: color("--tron-graph-label-fg", "--color-on-surface"),
    selectedNodeStroke: color("--tron-graph-selected", "--color-primary"),
  };
}

const floatingCanvasPanelStyle = {
  backgroundColor: "color-mix(in srgb, var(--color-elevated-surface) 88%, transparent)",
};

function numberText(value: number, locale: string) {
  return new Intl.NumberFormat(locale).format(value);
}

function nodeKind(labels: string[]): NodeKind {
  if (labels.some((label) => label.includes("Code") || label.includes("Symbol") || label.includes("Method") || label.includes("Function"))) return "code";
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

function nodeRadius(node: PositionedNode) {
  if (node.degree >= 4) return 9;
  if (node.degree >= 2) return 7;
  return 5.5;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function stableHash(value: string) {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function pointX(point: Point) {
  return point[0] ?? 0;
}

function pointY(point: Point) {
  return point[1] ?? 0;
}

function zoomToScale(zoom: number) {
  return clamp(zoom / 100, 0.05, 2.2);
}

function scaleToZoom(scale: number) {
  return Math.round(clamp(scale, 0.05, 2.2) * 100);
}

function labelViewportScale(zoom: number) {
  return 1 / zoomToScale(zoom);
}

function edgeActive(edge: EdgeLike, selectedNodeCode: string, highlightedEdgeCodes: string[]) {
  return edge.sourceNodeCode === selectedNodeCode
    || edge.targetNodeCode === selectedNodeCode
    || highlightedEdgeCodes.includes(edge.edgeCode);
}

function propertyText(node: GraphNodeView) {
  const candidates = [
    node.properties.skillName,
    node.properties.sourceCode,
    node.properties.promptCode,
    node.properties.category,
    node.properties.provider,
    node.properties.resourceType,
    node.fullText?.split("\n")[0],
  ];
  return candidates.find((item) => typeof item === "string" && item.trim().length > 0)?.toString() ?? node.labels[0] ?? "Graph node";
}

function hasExplicitCoordinates(node: GraphNodeView) {
  return Number.isFinite(node.x) && Number.isFinite(node.y);
}

function compareNodesByGraphWeight(degreeByNode: Map<string, number>) {
  return (first: GraphNodeView, second: GraphNodeView) => {
    const degreeDelta = (degreeByNode.get(second.nodeCode) ?? 0) - (degreeByNode.get(first.nodeCode) ?? 0);
    if (degreeDelta !== 0) return degreeDelta;

    const scoreDelta = (second.score ?? 0) - (first.score ?? 0);
    if (scoreDelta !== 0) return scoreDelta;

    return first.nodeCode.localeCompare(second.nodeCode);
  };
}

function buildAdjacency(nodes: GraphNodeView[], edges: GraphEdgeView[]) {
  const visibleNodeCodes = new Set(nodes.map((node) => node.nodeCode));
  const adjacency = new Map<string, Set<string>>();
  for (const node of nodes) {
    adjacency.set(node.nodeCode, new Set());
  }

  for (const edge of edges) {
    if (!visibleNodeCodes.has(edge.sourceNodeCode) || !visibleNodeCodes.has(edge.targetNodeCode)) continue;
    adjacency.get(edge.sourceNodeCode)?.add(edge.targetNodeCode);
    adjacency.get(edge.targetNodeCode)?.add(edge.sourceNodeCode);
  }

  return adjacency;
}

function buildGraphComponents(nodes: GraphNodeView[], adjacency: Map<string, Set<string>>, degreeByNode: Map<string, number>) {
  const compareNodes = compareNodesByGraphWeight(degreeByNode);
  const nodeByCode = new Map(nodes.map((node) => [node.nodeCode, node]));
  const visited = new Set<string>();
  const components: GraphNodeView[][] = [];

  for (const start of [...nodes].sort(compareNodes)) {
    if (visited.has(start.nodeCode)) continue;

    const component: GraphNodeView[] = [];
    const queue = [start.nodeCode];
    visited.add(start.nodeCode);

    while (queue.length) {
      const nodeCode = queue.shift() ?? "";
      const node = nodeByCode.get(nodeCode);
      if (node) component.push(node);

      const neighbors = [...(adjacency.get(nodeCode) ?? [])]
        .map((neighborCode) => nodeByCode.get(neighborCode))
        .filter((neighbor): neighbor is GraphNodeView => Boolean(neighbor))
        .sort(compareNodes);

      for (const neighbor of neighbors) {
        if (visited.has(neighbor.nodeCode)) continue;
        visited.add(neighbor.nodeCode);
        queue.push(neighbor.nodeCode);
      }
    }

    components.push(component.sort(compareNodes));
  }

  return components.sort((first, second) => second.length - first.length || compareNodes(first[0], second[0]));
}

function buildDepthGroups(component: GraphNodeView[], adjacency: Map<string, Set<string>>, degreeByNode: Map<string, number>) {
  const compareNodes = compareNodesByGraphWeight(degreeByNode);
  const componentNodeCodes = new Set(component.map((node) => node.nodeCode));
  const nodeByCode = new Map(component.map((node) => [node.nodeCode, node]));
  const depthByNode = new Map<string, number>();
  const root = [...component].sort(compareNodes)[0];
  const queue = [root.nodeCode];
  depthByNode.set(root.nodeCode, 0);

  while (queue.length) {
    const nodeCode = queue.shift() ?? "";
    const depth = depthByNode.get(nodeCode) ?? 0;
    const neighbors = [...(adjacency.get(nodeCode) ?? [])]
      .filter((neighborCode) => componentNodeCodes.has(neighborCode))
      .map((neighborCode) => nodeByCode.get(neighborCode))
      .filter((neighbor): neighbor is GraphNodeView => Boolean(neighbor))
      .sort(compareNodes);

    for (const neighbor of neighbors) {
      if (depthByNode.has(neighbor.nodeCode)) continue;
      depthByNode.set(neighbor.nodeCode, depth + 1);
      queue.push(neighbor.nodeCode);
    }
  }

  const groups = new Map<number, GraphNodeView[]>();
  for (const node of component) {
    const depth = depthByNode.get(node.nodeCode) ?? 0;
    groups.set(depth, [...(groups.get(depth) ?? []), node]);
  }

  return [...groups.entries()]
    .sort(([firstDepth], [secondDepth]) => firstDepth - secondDepth)
    .map(([depth, group]) => [depth, group.sort(compareNodes)] as const);
}

function buildGeneratedLayout(nodes: GraphNodeView[], edges: GraphEdgeView[], degreeByNode: Map<string, number>) {
  const layout = new Map<string, ViewportPoint>();
  const adjacency = buildAdjacency(nodes, edges);
  const components = buildGraphComponents(nodes, adjacency, degreeByNode);
  const componentOrbitRadius = components.length <= 1 ? 0 : Math.max(430, components.length * 92);

  components.forEach((component, componentIndex) => {
    const componentAngle = -Math.PI / 2 + (componentIndex / Math.max(1, components.length)) * Math.PI * 2;
    const componentCenter = {
      x: Math.cos(componentAngle) * componentOrbitRadius,
      y: Math.sin(componentAngle) * componentOrbitRadius,
    };
    const depthGroups = buildDepthGroups(component, adjacency, degreeByNode);

    for (const [depth, group] of depthGroups) {
      if (depth === 0 && group.length === 1) {
        layout.set(group[0].nodeCode, componentCenter);
        continue;
      }

      const ringRadius = depth === 0
        ? 84
        : Math.max(148 + (depth - 1) * 142, group.length * 28);
      const angleStep = Math.PI * 2 / Math.max(1, group.length);
      const angleOffset = -Math.PI / 2 + componentIndex * 0.41 + depth * 0.29;

      group.forEach((node, nodeIndex) => {
        const angle = angleOffset + nodeIndex * angleStep;
        layout.set(node.nodeCode, {
          x: componentCenter.x + Math.cos(angle) * ringRadius,
          y: componentCenter.y + Math.sin(angle) * ringRadius,
        });
      });
    }
  });

  return layout;
}

function looksLikeCodeGraph(nodes: GraphNodeView[], edges: GraphEdgeView[]) {
  const codeNodeCount = nodes.filter((node) => nodeKind(node.labels) === "code").length;
  const codeRelationshipCount = edges.filter((edge) => /CALL|DEPEND|IMPORT|REFERENCE|EXTEND|IMPLEMENT|DEFINE|CONTAIN/i.test(edge.relationship)).length;

  return codeNodeCount >= Math.max(3, nodes.length * 0.28) || codeRelationshipCount >= Math.max(3, edges.length * 0.36);
}

function resolveGraphLayoutMode(layoutMode: GraphLayoutMode, nodes: GraphNodeView[], edges: GraphEdgeView[]): ResolvedGraphLayoutMode {
  if (layoutMode !== "Smart") return layoutMode;
  return looksLikeCodeGraph(nodes, edges) ? "Hierarchy" : "Force";
}

function buildForceLayout(nodes: GraphNodeView[], edges: GraphEdgeView[], degreeByNode: Map<string, number>) {
  if (nodes.length > 140) return buildGeneratedLayout(nodes, edges, degreeByNode);

  const layout = new Map<string, ViewportPoint>();
  const adjacency = buildAdjacency(nodes, edges);
  const components = buildGraphComponents(nodes, adjacency, degreeByNode);
  const componentOrbitRadius = components.length <= 1 ? 0 : Math.max(430, components.length * 96);

  components.forEach((component, componentIndex) => {
    const componentAngle = -Math.PI / 2 + (componentIndex / Math.max(1, components.length)) * Math.PI * 2;
    const center = {
      x: Math.cos(componentAngle) * componentOrbitRadius,
      y: Math.sin(componentAngle) * componentOrbitRadius,
    };
    const positions = new Map<string, ViewportPoint>();
    const velocity = new Map<string, ViewportPoint>();
    const componentNodeCodes = new Set(component.map((node) => node.nodeCode));
    const componentEdges = edges.filter((edge) => componentNodeCodes.has(edge.sourceNodeCode) && componentNodeCodes.has(edge.targetNodeCode));
    const initialRadius = Math.max(145, Math.sqrt(component.length) * 62);

    component.forEach((node, nodeIndex) => {
      const angle = -Math.PI / 2 + (nodeIndex / Math.max(1, component.length)) * Math.PI * 2 + (stableHash(node.nodeCode) % 29) / 120;
      positions.set(node.nodeCode, {
        x: center.x + Math.cos(angle) * initialRadius,
        y: center.y + Math.sin(angle) * initialRadius,
      });
      velocity.set(node.nodeCode, { x: 0, y: 0 });
    });

    for (let iteration = 0; iteration < 120; iteration += 1) {
      const forces = new Map<string, ViewportPoint>();
      component.forEach((node) => forces.set(node.nodeCode, { x: 0, y: 0 }));

      for (let firstIndex = 0; firstIndex < component.length; firstIndex += 1) {
        for (let secondIndex = firstIndex + 1; secondIndex < component.length; secondIndex += 1) {
          const first = component[firstIndex];
          const second = component[secondIndex];
          const firstPosition = positions.get(first.nodeCode);
          const secondPosition = positions.get(second.nodeCode);
          if (!firstPosition || !secondPosition) continue;

          const dx = firstPosition.x - secondPosition.x;
          const dy = firstPosition.y - secondPosition.y;
          const distance = Math.max(28, Math.hypot(dx, dy));
          const force = 4600 / (distance * distance);
          const fx = (dx / distance) * force;
          const fy = (dy / distance) * force;
          const firstForce = forces.get(first.nodeCode);
          const secondForce = forces.get(second.nodeCode);
          if (!firstForce || !secondForce) continue;

          firstForce.x += fx;
          firstForce.y += fy;
          secondForce.x -= fx;
          secondForce.y -= fy;
        }
      }

      for (const edge of componentEdges) {
        const sourcePosition = positions.get(edge.sourceNodeCode);
        const targetPosition = positions.get(edge.targetNodeCode);
        const sourceForce = forces.get(edge.sourceNodeCode);
        const targetForce = forces.get(edge.targetNodeCode);
        if (!sourcePosition || !targetPosition || !sourceForce || !targetForce) continue;

        const dx = targetPosition.x - sourcePosition.x;
        const dy = targetPosition.y - sourcePosition.y;
        const distance = Math.max(1, Math.hypot(dx, dy));
        const targetDistance = 158;
        const force = (distance - targetDistance) * 0.028;
        const fx = (dx / distance) * force;
        const fy = (dy / distance) * force;
        sourceForce.x += fx;
        sourceForce.y += fy;
        targetForce.x -= fx;
        targetForce.y -= fy;
      }

      for (const node of component) {
        const position = positions.get(node.nodeCode);
        const nodeVelocity = velocity.get(node.nodeCode);
        const force = forces.get(node.nodeCode);
        if (!position || !nodeVelocity || !force) continue;

        force.x += (center.x - position.x) * 0.014;
        force.y += (center.y - position.y) * 0.014;
        nodeVelocity.x = (nodeVelocity.x + force.x) * 0.78;
        nodeVelocity.y = (nodeVelocity.y + force.y) * 0.78;
        position.x += clamp(nodeVelocity.x, -12, 12);
        position.y += clamp(nodeVelocity.y, -12, 12);
      }
    }

    for (const node of component) {
      const position = positions.get(node.nodeCode);
      if (position) layout.set(node.nodeCode, position);
    }
  });

  return layout;
}

function buildHierarchyLayout(nodes: GraphNodeView[], edges: GraphEdgeView[], degreeByNode: Map<string, number>) {
  const layout = new Map<string, ViewportPoint>();
  const adjacency = buildAdjacency(nodes, edges);
  const components = buildGraphComponents(nodes, adjacency, degreeByNode);
  const componentGap = 260;
  let nextComponentTop = 0;

  components.forEach((component) => {
    const depthGroups = buildDepthGroups(component, adjacency, degreeByNode);
    const maxGroupSize = Math.max(1, ...depthGroups.map(([, group]) => group.length));
    const componentHeight = Math.max(220, (maxGroupSize - 1) * 92);
    const componentTop = nextComponentTop;
    const componentCenterY = componentTop + componentHeight / 2;
    const columnGap = 210;
    const rowGap = 92;

    depthGroups.forEach(([depth, group]) => {
      const groupHeight = (group.length - 1) * rowGap;
      const startY = componentCenterY - groupHeight / 2;
      group.forEach((node, nodeIndex) => {
        layout.set(node.nodeCode, {
          x: (depth - (depthGroups.length - 1) / 2) * columnGap,
          y: startY + nodeIndex * rowGap,
        });
      });
    });

    nextComponentTop += componentHeight + componentGap;
  });

  const yOffset = (nextComponentTop - componentGap) / 2;
  for (const [nodeCode, point] of layout) {
    layout.set(nodeCode, {
      x: point.x,
      y: point.y - yOffset,
    });
  }

  return layout;
}

function buildGridLayout(nodes: GraphNodeView[], degreeByNode: Map<string, number>) {
  const layout = new Map<string, ViewportPoint>();
  const sortedNodes = [...nodes].sort(compareNodesByGraphWeight(degreeByNode));
  const columns = Math.max(1, Math.ceil(Math.sqrt(sortedNodes.length * 1.35)));
  const rows = Math.max(1, Math.ceil(sortedNodes.length / columns));
  const columnGap = 148;
  const rowGap = 108;

  sortedNodes.forEach((node, nodeIndex) => {
    const column = nodeIndex % columns;
    const row = Math.floor(nodeIndex / columns);
    layout.set(node.nodeCode, {
      x: (column - (columns - 1) / 2) * columnGap,
      y: (row - (rows - 1) / 2) * rowGap,
    });
  });

  return layout;
}

function edgesShareEndpoint(first: EdgeLike, second: EdgeLike) {
  return first.sourceNodeCode === second.sourceNodeCode
    || first.sourceNodeCode === second.targetNodeCode
    || first.targetNodeCode === second.sourceNodeCode
    || first.targetNodeCode === second.targetNodeCode;
}

function crossProduct(origin: ViewportPoint, first: ViewportPoint, second: ViewportPoint) {
  return (first.x - origin.x) * (second.y - origin.y) - (first.y - origin.y) * (second.x - origin.x);
}

function segmentsCross(
  firstStart: ViewportPoint,
  firstEnd: ViewportPoint,
  secondStart: ViewportPoint,
  secondEnd: ViewportPoint,
) {
  const epsilon = 0.001;
  const firstSideA = crossProduct(firstStart, firstEnd, secondStart);
  const firstSideB = crossProduct(firstStart, firstEnd, secondEnd);
  const secondSideA = crossProduct(secondStart, secondEnd, firstStart);
  const secondSideB = crossProduct(secondStart, secondEnd, firstEnd);

  return ((firstSideA > epsilon && firstSideB < -epsilon) || (firstSideA < -epsilon && firstSideB > epsilon))
    && ((secondSideA > epsilon && secondSideB < -epsilon) || (secondSideA < -epsilon && secondSideB > epsilon));
}

function layoutEdgeCrossingCount(layout: Map<string, ViewportPoint>, edges: EdgeLike[]) {
  let count = 0;

  for (let firstIndex = 0; firstIndex < edges.length; firstIndex += 1) {
    for (let secondIndex = firstIndex + 1; secondIndex < edges.length; secondIndex += 1) {
      const first = edges[firstIndex];
      const second = edges[secondIndex];
      if (edgesShareEndpoint(first, second)) continue;

      const firstSource = layout.get(first.sourceNodeCode);
      const firstTarget = layout.get(first.targetNodeCode);
      const secondSource = layout.get(second.sourceNodeCode);
      const secondTarget = layout.get(second.targetNodeCode);
      if (!firstSource || !firstTarget || !secondSource || !secondTarget) continue;

      if (segmentsCross(firstSource, firstTarget, secondSource, secondTarget)) count += 1;
    }
  }

  return count;
}

function layoutEdgeLength(layout: Map<string, ViewportPoint>, edges: EdgeLike[]) {
  return edges.reduce((total, edge) => {
    const source = layout.get(edge.sourceNodeCode);
    const target = layout.get(edge.targetNodeCode);
    if (!source || !target) return total;
    return total + Math.hypot(target.x - source.x, target.y - source.y);
  }, 0);
}

function layoutScore(layout: Map<string, ViewportPoint>, edges: EdgeLike[]) {
  return layoutEdgeCrossingCount(layout, edges) * 10000 + layoutEdgeLength(layout, edges) * 0.01;
}

function reduceLayoutEdgeCrossings(
  nodes: GraphNodeView[],
  edges: GraphEdgeView[],
  layout: Map<string, ViewportPoint>,
  degreeByNode: Map<string, number>,
) {
  const visibleEdges = edges.filter((edge) => layout.has(edge.sourceNodeCode) && layout.has(edge.targetNodeCode));
  if (nodes.length < 4 || visibleEdges.length < 3 || nodes.length > 90 || visibleEdges.length > 160) return layout;

  const optimizedLayout = new Map(layout);
  const adjacency = buildAdjacency(nodes, visibleEdges);
  const components = buildGraphComponents(nodes, adjacency, degreeByNode);

  for (const component of components) {
    const componentNodeCodes = new Set(component.map((node) => node.nodeCode));
    const componentEdges = visibleEdges.filter((edge) => componentNodeCodes.has(edge.sourceNodeCode) && componentNodeCodes.has(edge.targetNodeCode));
    if (component.length < 4 || componentEdges.length < 3) continue;

    const maxDegree = Math.max(...component.map((node) => degreeByNode.get(node.nodeCode) ?? 0));
    const lockedNodeCodes = new Set(
      component
        .filter((node) => (degreeByNode.get(node.nodeCode) ?? 0) >= Math.max(4, maxDegree))
        .map((node) => node.nodeCode),
    );
    const movableNodes = component
      .filter((node) => !lockedNodeCodes.has(node.nodeCode))
      .sort((first, second) => first.nodeCode.localeCompare(second.nodeCode));
    let bestScore = layoutScore(optimizedLayout, componentEdges);

    for (let pass = 0; pass < 4; pass += 1) {
      let improved = false;

      for (let firstIndex = 0; firstIndex < movableNodes.length; firstIndex += 1) {
        for (let secondIndex = firstIndex + 1; secondIndex < movableNodes.length; secondIndex += 1) {
          const first = movableNodes[firstIndex];
          const second = movableNodes[secondIndex];
          const firstDegree = degreeByNode.get(first.nodeCode) ?? 0;
          const secondDegree = degreeByNode.get(second.nodeCode) ?? 0;
          if (Math.abs(firstDegree - secondDegree) > 2) continue;

          const firstPoint = optimizedLayout.get(first.nodeCode);
          const secondPoint = optimizedLayout.get(second.nodeCode);
          if (!firstPoint || !secondPoint) continue;

          optimizedLayout.set(first.nodeCode, secondPoint);
          optimizedLayout.set(second.nodeCode, firstPoint);
          const candidateScore = layoutScore(optimizedLayout, componentEdges);

          if (candidateScore + 0.5 < bestScore) {
            bestScore = candidateScore;
            improved = true;
          } else {
            optimizedLayout.set(first.nodeCode, firstPoint);
            optimizedLayout.set(second.nodeCode, secondPoint);
          }
        }
      }

      if (!improved) break;
    }
  }

  return optimizedLayout;
}

function buildLayout(nodes: GraphNodeView[], edges: GraphEdgeView[], degreeByNode: Map<string, number>, layoutMode: ResolvedGraphLayoutMode) {
  if (layoutMode === "Force") {
    return reduceLayoutEdgeCrossings(nodes, edges, buildForceLayout(nodes, edges, degreeByNode), degreeByNode);
  }

  if (layoutMode === "Hierarchy") return buildHierarchyLayout(nodes, edges, degreeByNode);
  if (layoutMode === "Grid") return buildGridLayout(nodes, degreeByNode);

  return reduceLayoutEdgeCrossings(nodes, edges, buildGeneratedLayout(nodes, edges, degreeByNode), degreeByNode);
}

function buildPositionedNodes(nodes: GraphNodeView[], edges: GraphEdgeView[], layoutMode: GraphLayoutMode) {
  const degreeByNode = new Map<string, number>();
  for (const edge of edges) {
    degreeByNode.set(edge.sourceNodeCode, (degreeByNode.get(edge.sourceNodeCode) ?? 0) + 1);
    degreeByNode.set(edge.targetNodeCode, (degreeByNode.get(edge.targetNodeCode) ?? 0) + 1);
  }

  const allNodesHaveCoordinates = nodes.length > 0 && nodes.every(hasExplicitCoordinates);
  const resolvedLayoutMode = resolveGraphLayoutMode(layoutMode, nodes, edges);
  const useExplicitCoordinates = allNodesHaveCoordinates && layoutMode === "Smart";
  const generatedLayout = useExplicitCoordinates ? undefined : buildLayout(nodes, edges, degreeByNode, resolvedLayoutMode);

  return nodes.map<PositionedNode>((node) => {
    const colorGroup = graphNodeColorGroup(node.labels);
    const generatedPoint = generatedLayout?.get(node.nodeCode);
    return {
      ...node,
      colorGroup,
      degree: degreeByNode.get(node.nodeCode) ?? 0,
      layoutX: useExplicitCoordinates ? node.x ?? 0 : generatedPoint?.x ?? 0,
      layoutY: useExplicitCoordinates ? node.y ?? 0 : generatedPoint?.y ?? 0,
    };
  });
}

function buildGraphBounds(nodes: PositionedNode[]): GraphBounds {
  if (!nodes.length) {
    return {
      height: 1,
      maxX: 1,
      maxY: 1,
      minX: 0,
      minY: 0,
      width: 1,
    };
  }

  const xValues = nodes.map((node) => node.layoutX);
  const yValues = nodes.map((node) => node.layoutY);
  const minX = Math.min(...xValues);
  const maxX = Math.max(...xValues);
  const minY = Math.min(...yValues);
  const maxY = Math.max(...yValues);
  const rawWidth = Math.max(1, maxX - minX);
  const rawHeight = Math.max(1, maxY - minY);
  const padding = Math.max(80, Math.max(rawWidth, rawHeight) * 0.16);

  return {
    height: rawHeight + padding * 2,
    maxX: maxX + padding,
    maxY: maxY + padding,
    minX: minX - padding,
    minY: minY - padding,
    width: rawWidth + padding * 2,
  };
}

function graphPointForNode(node: PositionedNode): ViewportPoint {
  return {
    x: node.layoutX,
    y: node.layoutY,
  };
}

function graphCenter(bounds: GraphBounds): ViewportPoint {
  return {
    x: bounds.minX + bounds.width / 2,
    y: bounds.minY + bounds.height / 2,
  };
}

function minimapNodePosition(node: PositionedNode, bounds: GraphBounds) {
  const point = graphPointForNode(node);
  return {
    left: `${clamp(((point.x - bounds.minX) / bounds.width) * 100, 4, 96)}%`,
    top: `${clamp(((point.y - bounds.minY) / bounds.height) * 100, 4, 96)}%`,
  };
}

function minimapVisibleSizeRatio(visibleMin: number, visibleMax: number, boundsSize: number) {
  return clamp((visibleMax - visibleMin) / boundsSize, 0.001, 1);
}

function minimapDisplaySizeRatio(visibleSizeRatio: number) {
  return Math.min(1, Math.max(0.08, visibleSizeRatio));
}

function minimapDisplayStartRatio(centerRatio: number, displaySizeRatio: number) {
  return displaySizeRatio >= 1
    ? 0
    : clamp(centerRatio - displaySizeRatio / 2, 0, 1 - displaySizeRatio);
}

function minimapContentCenterRatio(displayStartRatio: number, displaySizeRatio: number, visibleSizeRatio: number) {
  if (displaySizeRatio >= 1) return 0.5;
  if (displayStartRatio <= 0.0001) return visibleSizeRatio / 2;
  if (displayStartRatio >= 1 - displaySizeRatio - 0.0001) return 1 - visibleSizeRatio / 2;

  return clamp(displayStartRatio + displaySizeRatio / 2, visibleSizeRatio / 2, 1 - visibleSizeRatio / 2);
}

function buildMinimapViewport(graph: Graph, bounds: GraphBounds): MinimapViewport {
  const [width, height] = graph.getSize();
  if (!width || !height) return fallbackMinimapViewport;

  const topLeft = graph.getCanvasByViewport([0, 0]);
  const bottomRight = graph.getCanvasByViewport([width, height]);
  const visibleMinX = Math.min(pointX(topLeft), pointX(bottomRight));
  const visibleMaxX = Math.max(pointX(topLeft), pointX(bottomRight));
  const visibleMinY = Math.min(pointY(topLeft), pointY(bottomRight));
  const visibleMaxY = Math.max(pointY(topLeft), pointY(bottomRight));
  const visibleWidthRatio = minimapVisibleSizeRatio(visibleMinX, visibleMaxX, bounds.width);
  const visibleHeightRatio = minimapVisibleSizeRatio(visibleMinY, visibleMaxY, bounds.height);
  const displayWidthRatio = minimapDisplaySizeRatio(visibleWidthRatio);
  const displayHeightRatio = minimapDisplaySizeRatio(visibleHeightRatio);
  const centerXRatio = clamp(((visibleMinX + visibleMaxX) / 2 - bounds.minX) / bounds.width, 0, 1);
  const centerYRatio = clamp(((visibleMinY + visibleMaxY) / 2 - bounds.minY) / bounds.height, 0, 1);
  const leftRatio = minimapDisplayStartRatio(centerXRatio, displayWidthRatio);
  const topRatio = minimapDisplayStartRatio(centerYRatio, displayHeightRatio);

  return {
    height: displayHeightRatio * 100,
    left: leftRatio * 100,
    top: topRatio * 100,
    visibleHeight: visibleHeightRatio * 100,
    visibleWidth: visibleWidthRatio * 100,
    width: displayWidthRatio * 100,
  };
}

function edgeCurveOffset(source: ViewportPoint, target: ViewportPoint, active: boolean, center: ViewportPoint) {
  const dx = target.x - source.x;
  const dy = target.y - source.y;
  const length = Math.hypot(dx, dy) || 1;
  const normal = { x: -dy / length, y: dx / length };
  const midpoint = { x: (source.x + target.x) / 2, y: (source.y + target.y) / 2 };
  const outward = normal.x * (midpoint.x - center.x) + normal.y * (midpoint.y - center.y);
  const direction = Math.abs(outward) > 0.001 ? Math.sign(outward) : (source.x <= target.x ? -1 : 1);
  const offset = Math.min(active ? 78 : 48, Math.max(active ? 26 : 14, length * (active ? 0.13 : 0.07)));

  return offset * direction;
}

function buildNodeZoomStyle(node: PositionedNode, selected: boolean, highlighted: boolean, zoom: number) {
  const labelScale = labelViewportScale(zoom);
  const radius = nodeRadius(node) + (selected ? 2 : highlighted ? 1 : 0);

  return {
    r: radius * labelScale,
    size: radius * 2 * labelScale,
    lineWidth: (selected ? 2.5 : highlighted ? 1.8 : 0) * labelScale,
    label: true,
    labelFontSize: (selected || highlighted ? 13 : 12) * labelScale,
    labelOffsetX: 7 * labelScale,
  };
}

function buildEdgeZoomStyle(edge: EdgeLike, active: boolean, zoom: number) {
  const labelScale = labelViewportScale(zoom);

  return {
    lineWidth: (active ? 1.35 : 0.8) * labelScale,
    label: active,
    labelBackgroundLineWidth: labelScale,
    labelBackgroundRadius: 10 * labelScale,
    labelFontSize: 10 * labelScale,
    labelPadding: [2 * labelScale, 8 * labelScale],
  };
}

function buildG6Data(
  nodes: PositionedNode[],
  edges: GraphEdgeView[],
  bounds: GraphBounds,
  selectedNodeCode: string,
  highlightedNodeCodes: string[],
  highlightedEdgeCodes: string[],
  zoom: number,
  nodeLabelProperty: string,
  graphCanvasTheme: GraphCanvasTheme,
): GraphData {
  const highlightedNodes = new Set(highlightedNodeCodes);
  const nodeByCode = new Map(nodes.map((node) => [node.nodeCode, node]));
  const center = graphCenter(bounds);

  return {
    nodes: nodes.map((node) => {
      const selected = node.nodeCode === selectedNodeCode;
      const highlighted = selected || highlightedNodes.has(node.nodeCode);
      const palette = graphNodeColorForGroup(node.colorGroup);
      const point = graphPointForNode(node);
      return {
        id: node.nodeCode,
        data: {
          colorGroup: node.colorGroup,
          degree: node.degree,
          detail: propertyText(node),
          nodeCode: node.nodeCode,
          primaryLabel: node.labels[0] ?? "GraphNode",
        },
        states: [],
        style: {
          cursor: "pointer",
          fill: palette.color,
          halo: false,
          labelBackground: false,
          labelFill: graphCanvasTheme.nodeLabelFill,
          labelFontFamily: "Inter Variable, Inter, sans-serif",
          labelFontWeight: 700,
          labelPadding: 0,
          labelPlacement: "right",
          labelText: resolveGraphNodeLabel(node, nodeLabelProperty),
          opacity: selected || highlighted ? 1 : 0.92,
          stroke: selected ? graphCanvasTheme.selectedNodeStroke : highlighted ? graphCanvasTheme.highlightedNodeStroke : "transparent",
          x: point.x,
          y: point.y,
          zIndex: selected ? 10 : highlighted ? 6 : Math.max(1, Math.min(5, node.degree)),
          ...buildNodeZoomStyle(node, selected, highlighted, zoom),
        },
        type: "circle",
      };
    }),
    edges: edges.map((edge) => {
      const source = nodeByCode.get(edge.sourceNodeCode);
      const target = nodeByCode.get(edge.targetNodeCode);
      const active = edgeActive(edge, selectedNodeCode, highlightedEdgeCodes);
      const sourcePoint = source ? graphPointForNode(source) : graphCenter(bounds);
      const targetPoint = target ? graphPointForNode(target) : graphCenter(bounds);
      return {
        id: edge.edgeCode,
        data: {
          edgeCode: edge.edgeCode,
          relationship: edge.relationship,
          sourceNodeCode: edge.sourceNodeCode,
          targetNodeCode: edge.targetNodeCode,
        },
        source: edge.sourceNodeCode,
        states: [],
        style: {
          badge: false,
          curveOffset: edgeCurveOffset(sourcePoint, targetPoint, active, center),
          endArrow: active,
          endArrowFill: graphCanvasTheme.activeEdgeColor,
          endArrowStroke: graphCanvasTheme.activeEdgeColor,
          labelAutoRotate: false,
          labelBackground: true,
          labelBackgroundFill: graphCanvasTheme.edgeLabelBackgroundFill,
          labelBackgroundOpacity: graphCanvasTheme.edgeLabelBackgroundOpacity,
          labelBackgroundStroke: graphCanvasTheme.edgeLabelBackgroundStroke,
          labelFill: graphCanvasTheme.activeEdgeColor,
          labelFontFamily: "Inter Variable, Inter, sans-serif",
          labelFontWeight: 700,
          labelOffsetX: 0,
          labelOffsetY: 0,
          labelPlacement: "center",
          labelText: edge.relationship,
          labelTextBaseline: "middle",
          opacity: active ? 0.72 : 0.38,
          stroke: active ? graphCanvasTheme.activeEdgeColor : graphCanvasTheme.edgeStroke,
          zIndex: active ? 4 : 1,
          ...buildEdgeZoomStyle(edge, active, zoom),
        },
        target: edge.targetNodeCode,
        type: "quadratic",
      };
    }),
  };
}

function buildG6ZoomStyleData(
  nodes: PositionedNode[],
  edges: GraphEdgeView[],
  selectedNodeCode: string,
  highlightedNodeCodes: string[],
  highlightedEdgeCodes: string[],
  zoom: number,
): GraphZoomStyleData {
  const highlightedNodes = new Set(highlightedNodeCodes);

  return {
    edges: edges.map((edge) => {
      const active = edgeActive(edge, selectedNodeCode, highlightedEdgeCodes);
      return {
        id: edge.edgeCode,
        source: edge.sourceNodeCode,
        style: buildEdgeZoomStyle(edge, active, zoom),
        target: edge.targetNodeCode,
      };
    }),
    nodes: nodes.map((node) => {
      const selected = node.nodeCode === selectedNodeCode;
      const highlighted = selected || highlightedNodes.has(node.nodeCode);
      return {
        id: node.nodeCode,
        style: buildNodeZoomStyle(node, selected, highlighted, zoom),
      };
    }),
  };
}

function centerGraphAt(graph: Graph, point: ViewportPoint, duration: number) {
  const [width, height] = graph.getSize();
  const viewportPosition = graph.getViewportByCanvas([point.x, point.y]);

  return graph.translateBy([
    width / 2 - pointX(viewportPosition),
    height / 2 - pointY(viewportPosition),
  ], duration ? { duration } : false);
}

async function fitGraphBounds(graph: Graph, bounds: GraphBounds) {
  const [width, height] = graph.getSize();
  const scale = clamp(Math.min((width - 64) / bounds.width, (height - 64) / bounds.height), 0.05, 1);
  await graph.zoomTo(scale, false, graphViewportCenter(graph));
  await centerGraphAt(graph, graphCenter(bounds), 0);
}

function buildGraphDataSyncKey(
  nodes: PositionedNode[],
  edges: GraphEdgeView[],
  bounds: GraphBounds,
  selectedNodeCode: string,
  highlightedNodeCodes: string[],
  highlightedEdgeCodes: string[],
  nodeLabelProperty: string,
  theme: string,
) {
  return [
    bounds.height,
    bounds.minX,
    bounds.minY,
    bounds.width,
    highlightedEdgeCodes.join(","),
    highlightedNodeCodes.join(","),
    nodeLabelProperty,
    selectedNodeCode,
    theme,
    nodes.map((node) => [
      node.nodeCode,
      node.layoutX,
      node.layoutY,
      node.labels.join("/"),
      resolveGraphNodeLabel(node, nodeLabelProperty),
    ].join(":")).join(";"),
    edges.map((edge) => `${edge.edgeCode}:${edge.relationship}`).join(";"),
  ].join("|");
}

function graphViewportCenter(graph: Graph): Point {
  const [width, height] = graph.getSize();
  return [width / 2, height / 2];
}

function viewportPointFromEvent(event: IElementEvent): Point | undefined {
  const viewport = (event as IElementEvent & { viewport?: { x?: number; y?: number } }).viewport;
  if (typeof viewport?.x !== "number" || typeof viewport.y !== "number") return undefined;
  return [viewport.x, viewport.y];
}

async function centerGraphNode(graph: Graph, nodeCode: string, duration: number) {
  const [width, height] = graph.getSize();
  if (!width || !height) return;

  try {
    const nodePosition = graph.getElementPosition(nodeCode);
    const viewportPosition = graph.getViewportByCanvas(nodePosition);
    await graph.translateBy([
      width / 2 - pointX(viewportPosition),
      height / 2 - pointY(viewportPosition),
    ], duration ? { duration } : false);
  } catch {
    // Fall back to G6's element focus when the rendered node position is not ready yet.
    await graph.focusElement(nodeCode, duration ? { duration } : false);
  }
}

function nodeCodeFromEvent(event: IElementEvent) {
  const target = event.target as { id?: string };
  return target.id ?? "";
}

export function GraphCanvas({
  edges,
  highlightedEdgeCodes,
  highlightedNodeCodes,
  layoutMode,
  metrics,
  nodeLabelProperty,
  nodes,
  onCameraZoomChange,
  onClearSelection,
  onSelectNode,
  selectedNodeCode,
  zoom,
}: GraphCanvasProps) {
  const { i18n, t } = useTranslation(["pages"]);
  const { activeSkin } = useTheme();
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:knowledgeGraph.${key}`, { defaultValue, ...options })
  );
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const graphCanvasTheme = useMemo(() => readGraphCanvasTheme(), [activeSkin]);
  const theme = JSON.stringify(activeSkin);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const minimapRef = useRef<HTMLButtonElement | null>(null);
  const minimapDragRef = useRef<MinimapDragState | null>(null);
  const graphRef = useRef<Graph | null>(null);
  const pendingGraphTasksRef = useRef(new WeakMap<Graph, Set<Promise<unknown>>>());
  const syncFrameRef = useRef<number | null>(null);
  const onCameraZoomChangeRef = useRef(onCameraZoomChange);
  const onClearSelectionRef = useRef(onClearSelection);
  const onSelectNodeRef = useRef(onSelectNode);
  const suppressZoomReportRef = useRef(false);
  const zoomRef = useRef(zoom);
  const cameraZoomRef = useRef(zoom);
  const zoomStyleDataRef = useRef<GraphZoomStyleData>({ edges: [], nodes: [] });
  const graphReadyRef = useRef(false);
  const fittedNodesRef = useRef(new Set<string>());
  const [cameraZoom, setCameraZoom] = useState(zoom);
  const [minimapViewport, setMinimapViewport] = useState<MinimapViewport>(fallbackMinimapViewport);

  const positionedNodes = useMemo(
    () => buildPositionedNodes(nodes, edges, layoutMode),
    [edges, layoutMode, nodes],
  );
  const graphBounds = useMemo(
    () => buildGraphBounds(positionedNodes),
    [positionedNodes],
  );
  const nodeByCode = useMemo(
    () => new Map(positionedNodes.map((node) => [node.nodeCode, node])),
    [positionedNodes],
  );
  const visibleNodeCodes = useMemo(
    () => new Set(positionedNodes.map((node) => node.nodeCode)),
    [positionedNodes],
  );
  const nodeLabelPreview = useMemo(
    () => positionedNodes.map((node) => resolveGraphNodeLabel(node, nodeLabelProperty)).join(" | "),
    [nodeLabelProperty, positionedNodes],
  );
  const legendItems = useMemo(
    () => graphNodeLegendItems(positionedNodes),
    [positionedNodes, activeSkin],
  );
  const renderEdges = useMemo(
    () => edges.filter((edge) => visibleNodeCodes.has(edge.sourceNodeCode) && visibleNodeCodes.has(edge.targetNodeCode)),
    [edges, visibleNodeCodes],
  );
  const g6Data = useMemo(
    () => buildG6Data(positionedNodes, renderEdges, graphBounds, selectedNodeCode, highlightedNodeCodes, highlightedEdgeCodes, cameraZoomRef.current, nodeLabelProperty, graphCanvasTheme),
    [graphBounds, graphCanvasTheme, highlightedEdgeCodes, highlightedNodeCodes, nodeLabelProperty, positionedNodes, renderEdges, selectedNodeCode],
  );
  const g6DataSyncKey = useMemo(
    () => buildGraphDataSyncKey(positionedNodes, renderEdges, graphBounds, selectedNodeCode, highlightedNodeCodes, highlightedEdgeCodes, nodeLabelProperty, theme),
    [graphBounds, highlightedEdgeCodes, highlightedNodeCodes, nodeLabelProperty, positionedNodes, renderEdges, selectedNodeCode, theme],
  );
  const g6DataSyncRef = useRef({ bounds: graphBounds, data: g6Data, selectedNodeCode });
  g6DataSyncRef.current = { bounds: graphBounds, data: g6Data, selectedNodeCode };
  const zoomStyleData = useMemo(
    () => buildG6ZoomStyleData(positionedNodes, renderEdges, selectedNodeCode, highlightedNodeCodes, highlightedEdgeCodes, cameraZoom),
    [cameraZoom, highlightedEdgeCodes, highlightedNodeCodes, positionedNodes, renderEdges, selectedNodeCode],
  );

  function trackGraphTask(graph: Graph, task: () => Promise<unknown>) {
    let tasks = pendingGraphTasksRef.current.get(graph);
    if (!tasks) {
      tasks = new Set<Promise<unknown>>();
      pendingGraphTasksRef.current.set(graph, tasks);
    }

    const promise = Promise.resolve()
      .then(task)
      .catch(() => undefined);
    tasks.add(promise);
    void promise.finally(() => {
      tasks?.delete(promise);
    });
    return promise;
  }

  function destroyGraphWhenIdle(graph: Graph) {
    const tasks = pendingGraphTasksRef.current.get(graph);
    const destroy = () => {
      try {
        graph.destroy();
      } catch {
        // G6 can throw if a development remount already disposed the instance.
      }
      pendingGraphTasksRef.current.delete(graph);
    };

    if (!tasks?.size) {
      destroy();
      return;
    }

    void Promise.allSettled(Array.from(tasks)).finally(destroy);
  }

  function reportZoom(graph: Graph) {
    const nextZoom = scaleToZoom(graph.getZoom());
    cameraZoomRef.current = nextZoom;
    setCameraZoom((current) => (current === nextZoom ? current : nextZoom));
    if (!suppressZoomReportRef.current && Math.abs(nextZoom - zoomRef.current) >= 1) {
      zoomRef.current = nextZoom;
      onCameraZoomChangeRef.current(nextZoom);
    }
  }

  function syncViewportState(graph: Graph) {
    reportZoom(graph);
    setMinimapViewport(buildMinimapViewport(graph, graphBounds));
  }

  function scheduleSyncViewportState(targetGraph = graphRef.current) {
    const graph = targetGraph;
    if (!graph) return;
    if (syncFrameRef.current !== null) window.cancelAnimationFrame(syncFrameRef.current);
    syncFrameRef.current = window.requestAnimationFrame(() => {
      syncFrameRef.current = null;
      if (graphRef.current !== graph) return;
      syncViewportState(graph);
    });
  }

  function panCameraFromMinimapPointer(event: ReactPointerEvent<HTMLButtonElement>) {
    const graph = graphRef.current;
    const minimap = minimapRef.current;
    const dragState = minimapDragRef.current;
    if (!graph || !minimap || !dragState) return;

    const rect = minimap.getBoundingClientRect();
    const pointerXRatio = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    const pointerYRatio = clamp((event.clientY - rect.top) / rect.height, 0, 1);
    const viewportWidthRatio = clamp(minimapViewport.width / 100, 0.001, 1);
    const viewportHeightRatio = clamp(minimapViewport.height / 100, 0.001, 1);
    const visibleWidthRatio = clamp(minimapViewport.visibleWidth / 100, 0.001, 1);
    const visibleHeightRatio = clamp(minimapViewport.visibleHeight / 100, 0.001, 1);
    const targetLeftRatio = viewportWidthRatio >= 1
      ? 0
      : clamp(pointerXRatio - dragState.offsetXRatio, 0, 1 - viewportWidthRatio);
    const targetTopRatio = viewportHeightRatio >= 1
      ? 0
      : clamp(pointerYRatio - dragState.offsetYRatio, 0, 1 - viewportHeightRatio);
    const targetCenterXRatio = minimapContentCenterRatio(targetLeftRatio, viewportWidthRatio, visibleWidthRatio);
    const targetCenterYRatio = minimapContentCenterRatio(targetTopRatio, viewportHeightRatio, visibleHeightRatio);
    const targetCenter = {
      x: graphBounds.minX + targetCenterXRatio * graphBounds.width,
      y: graphBounds.minY + targetCenterYRatio * graphBounds.height,
    };

    void trackGraphTask(graph, async () => {
      await centerGraphAt(graph, targetCenter, 0);
      scheduleSyncViewportState(graph);
    });
  }

  function handleMinimapPointerDown(event: ReactPointerEvent<HTMLButtonElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    const pointerXRatio = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    const pointerYRatio = clamp((event.clientY - rect.top) / rect.height, 0, 1);
    const viewportLeftRatio = minimapViewport.left / 100;
    const viewportTopRatio = minimapViewport.top / 100;
    const viewportWidthRatio = clamp(minimapViewport.width / 100, 0.001, 1);
    const viewportHeightRatio = clamp(minimapViewport.height / 100, 0.001, 1);
    const pointerInsideViewport = pointerXRatio >= viewportLeftRatio
      && pointerXRatio <= viewportLeftRatio + viewportWidthRatio
      && pointerYRatio >= viewportTopRatio
      && pointerYRatio <= viewportTopRatio + viewportHeightRatio;

    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    minimapDragRef.current = {
      offsetXRatio: pointerInsideViewport ? pointerXRatio - viewportLeftRatio : viewportWidthRatio / 2,
      offsetYRatio: pointerInsideViewport ? pointerYRatio - viewportTopRatio : viewportHeightRatio / 2,
      pointerId: event.pointerId,
    };
    panCameraFromMinimapPointer(event);
  }

  function handleMinimapPointerMove(event: ReactPointerEvent<HTMLButtonElement>) {
    if (minimapDragRef.current?.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    panCameraFromMinimapPointer(event);
  }

  function handleMinimapPointerEnd(event: ReactPointerEvent<HTMLButtonElement>) {
    if (minimapDragRef.current?.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    minimapDragRef.current = null;
  }

  useEffect(() => {
    onCameraZoomChangeRef.current = onCameraZoomChange;
    onClearSelectionRef.current = onClearSelection;
    onSelectNodeRef.current = onSelectNode;
    zoomRef.current = zoom;
  }, [onCameraZoomChange, onClearSelection, onSelectNode, zoom]);

  useEffect(() => {
    zoomStyleDataRef.current = zoomStyleData;
  }, [zoomStyleData]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return undefined;

    const graph = new Graph({
      animation: false,
      background: "transparent",
      behaviors: [
        { direction: "both", key: "drag-canvas", range: Infinity, type: "drag-canvas" },
        { key: "zoom-canvas", preventDefault: true, sensitivity: 0.9, type: "zoom-canvas" },
      ],
      container,
      data: g6Data,
      edge: { type: "quadratic" },
      node: { type: "circle" },
      padding: 42,
      zoom: zoomToScale(zoom),
      zoomRange: [0.05, 2.2],
    });

    graphRef.current = graph;
    graphReadyRef.current = false;
    let active = true;

    const handleNodeClick = (event: IElementEvent) => {
      const nodeCode = nodeCodeFromEvent(event);
      if (!nodeCode) return;

      onSelectNodeRef.current(nodeCode);
    };
    const handleCanvasClick = () => {
      onClearSelectionRef.current();
    };

    const handleCanvasDoubleClick = (event: IElementEvent) => {
      void trackGraphTask(graph, async () => {
        // Canvas interactions zoom around the pointer. Toolbar buttons use the
        // explicit viewport center below, so the two interaction modes remain
        // predictable and do not unexpectedly move the graph under the cursor.
        await graph.zoomBy(1.28, { duration: 220 }, viewportPointFromEvent(event) ?? graphViewportCenter(graph));
        if (active) scheduleSyncViewportState(graph);
      });
    };
    const handleGraphTransform = () => {
      scheduleSyncViewportState();
    };
    const resizeObserver = new ResizeObserver(() => {
      graph.resize();
      scheduleSyncViewportState();
    });

    graph.on(NodeEvent.CLICK, handleNodeClick);
    graph.on(CanvasEvent.CLICK, handleCanvasClick);
    graph.on(CanvasEvent.DBLCLICK, handleCanvasDoubleClick);
    graph.on(GraphEvent.AFTER_TRANSFORM, handleGraphTransform);
    resizeObserver.observe(container);

    void trackGraphTask(graph, async () => {
      await graph.render();
      if (!active) return;
      graphReadyRef.current = true;
      const selectedNode = selectedNodeCode ? nodeByCode.get(selectedNodeCode) : undefined;
      if (selectedNode) await centerGraphAt(graph, graphPointForNode(selectedNode), 0);
      else if (nodes.length) await fitGraphBounds(graph, graphBounds);
      else await centerGraphAt(graph, graphCenter(graphBounds), 0);
      if (active) scheduleSyncViewportState(graph);
    });

    return () => {
      active = false;
      if (syncFrameRef.current !== null) {
        window.cancelAnimationFrame(syncFrameRef.current);
        syncFrameRef.current = null;
      }
      resizeObserver.disconnect();
      graph.off(NodeEvent.CLICK, handleNodeClick);
      graph.off(CanvasEvent.CLICK, handleCanvasClick);
      graph.off(CanvasEvent.DBLCLICK, handleCanvasDoubleClick);
      graph.off(GraphEvent.AFTER_TRANSFORM, handleGraphTransform);
      if (graphRef.current === graph) {
        graphRef.current = null;
        graphReadyRef.current = false;
      }
      destroyGraphWhenIdle(graph);
    };
    // The graph instance is intentionally created once per canvas mount.
    // Data, selection, zoom, and layout changes are synchronized in the effects below.
  }, []);

  useEffect(() => {
    const graph = graphRef.current;
    if (!graph) return;
    const { bounds, data, selectedNodeCode: currentSelectedNodeCode } = g6DataSyncRef.current;
    const nextIds = new Set((data.nodes ?? []).map((node) => String(node.id)));
    const previousIds = fittedNodesRef.current;
    const isNewWindow = nextIds.size > 0 && (previousIds.size === 0 || !Array.from(previousIds).every((id) => nextIds.has(id)));
    fittedNodesRef.current = nextIds;

    void trackGraphTask(graph, async () => {
      graph.setData(data);
      await graph.draw();
      if (graphRef.current === graph) {
        if (!currentSelectedNodeCode) {
          if (isNewWindow && g6DataSyncRef.current.data === data) {
            await fitGraphBounds(graph, bounds);
            reportZoom(graph);
          } else await centerGraphAt(graph, graphCenter(bounds), 0);
        }
        setMinimapViewport(buildMinimapViewport(graph, bounds));
      }
    });
  }, [g6DataSyncKey]);

  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || !graphReadyRef.current) return;

    void trackGraphTask(graph, async () => {
      const currentZoomStyleData = zoomStyleDataRef.current;
      graph.updateNodeData(currentZoomStyleData.nodes);
      graph.updateEdgeData(currentZoomStyleData.edges);
      await graph.draw();
      if (graphRef.current === graph) {
        setMinimapViewport(buildMinimapViewport(graph, graphBounds));
      }
    });
  }, [cameraZoom, graphBounds]);

  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || !graphReadyRef.current) return;
    const targetScale = zoomToScale(zoom);
    if (Math.abs(graph.getZoom() - targetScale) < 0.01) return;

    suppressZoomReportRef.current = true;
    void trackGraphTask(graph, async () => {
      try {
        // This effect is driven by the toolbar's +/- controls. Unlike pointer
        // interactions, toolbar zoom is intentionally anchored at the canvas
        // center.
        await graph.zoomTo(targetScale, { duration: 180 }, graphViewportCenter(graph));
      } finally {
        suppressZoomReportRef.current = false;
        if (graphRef.current === graph) {
          scheduleSyncViewportState(graph);
          reportZoom(graph);
        }
      }
    });
  }, [zoom]);

  useEffect(() => {
    const graph = graphRef.current;
    const selectedNode = selectedNodeCode ? nodeByCode.get(selectedNodeCode) : undefined;
    if (!graph || !graphReadyRef.current || !selectedNode) return;

    suppressZoomReportRef.current = true;
    void trackGraphTask(graph, async () => {
      try {
        await centerGraphNode(graph, selectedNode.nodeCode, 260);
      } finally {
        suppressZoomReportRef.current = false;
        if (graphRef.current === graph) {
          scheduleSyncViewportState(graph);
          reportZoom(graph);
        }
      }
    });
  }, [graphBounds, nodeByCode, selectedNodeCode]);

  return (
    <section
      className="relative h-full min-h-0 min-w-0 overflow-hidden bg-[var(--color-surface-container-lowest)]"
      aria-label={pageT("canvas.aria", "Knowledge graph")}
      style={{
        backgroundImage: graphCanvasTheme.canvasBackgroundImage,
        backgroundSize: "20px 20px, 100% 100%",
      }}
    >
      <div ref={containerRef} className="absolute inset-0 z-0" />
      <div className="sr-only" aria-label={pageT("canvas.nodeLabelsAria", "Graph node labels")}>{nodeLabelPreview}</div>

      <div
        className="absolute bottom-4 left-4 z-20 rounded-xl border border-[var(--color-card-border)] p-2 shadow-[0_10px_28px_rgba(15,23,42,0.08)] backdrop-blur-sm"
        style={floatingCanvasPanelStyle}
      >
        <button
          ref={minimapRef}
          aria-label={pageT("canvas.minimapAria", "Pan graph viewport")}
          className="relative block h-28 w-40 cursor-grab overflow-hidden rounded-lg bg-[var(--color-surface-container-low)] text-left active:cursor-grabbing"
          type="button"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
          }}
          onPointerCancel={handleMinimapPointerEnd}
          onPointerDown={handleMinimapPointerDown}
          onPointerMove={handleMinimapPointerMove}
          onPointerUp={handleMinimapPointerEnd}
        >
          {positionedNodes.map((node) => (
            <span
              className={cn(
                "absolute -translate-x-1/2 -translate-y-1/2 rounded-full",
                node.nodeCode === selectedNodeCode ? "size-2.5" : "size-1.5",
              )}
              key={`mini-${node.nodeCode}`}
              style={{
                backgroundColor: graphNodeColorForGroup(node.colorGroup).color,
                boxShadow: node.nodeCode === selectedNodeCode ? "0 0 0 2px color-mix(in srgb, var(--color-focus-ring) 28%, transparent)" : undefined,
                ...minimapNodePosition(node, graphBounds),
              }}
            />
          ))}
          <span
            className="absolute rounded-md border border-[var(--color-accent)]/60"
            style={{
              backgroundColor: "color-mix(in srgb, var(--color-on-surface) 20%, transparent)",
              boxShadow: "inset 0 0 0 1px color-mix(in srgb, var(--color-on-surface) 24%, transparent)",
              height: `${minimapViewport.height}%`,
              left: `${minimapViewport.left}%`,
              top: `${minimapViewport.top}%`,
              width: `${minimapViewport.width}%`,
            }}
          />
        </button>
      </div>

      <div className="absolute bottom-4 right-4 z-20 flex max-w-[min(42rem,calc(100%-1rem))] flex-col items-end gap-[12px]">
        <GraphLegend items={legendItems} />
        <div
          aria-label={pageT("canvas.metricsAria", "Graph canvas metrics")}
          className="flex items-center gap-[12px] rounded-xl border border-[var(--color-card-border)] px-3 py-1.5 font-mono text-[11px] text-[var(--color-on-surface-variant)] shadow-[0_10px_28px_rgba(15,23,42,0.08)] backdrop-blur-sm"
          style={floatingCanvasPanelStyle}
        >
          <span>{pageT("canvas.metrics.nodes", "{{count}} nodes", { count: numberText(metrics.nodeCount, locale) })}</span>
          <span className="text-[var(--color-outline)]">·</span>
          <span>{pageT("canvas.metrics.edges", "{{count}} edges", { count: numberText(metrics.edgeCount, locale) })}</span>
        </div>
      </div>
    </section>
  );
}
