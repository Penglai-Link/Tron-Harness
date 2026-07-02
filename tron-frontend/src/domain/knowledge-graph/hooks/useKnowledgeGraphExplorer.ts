import { useMemo } from "react";

import { knowledgeGraphApiEndpoints } from "@/domain/knowledge-graph/api";
import { getKnowledgeGraphMock } from "@/domain/knowledge-graph/mock/data";
import type {
  GraphEdgeView,
  GraphNodeView,
  KnowledgeGraphExplorerFilters,
  KnowledgeGraphExplorerView,
  KnowledgeGraphView,
} from "@/domain/knowledge-graph/model";
import { useServiceQuery } from "@/shared/api/serviceConfig";

function propertyValues(node: GraphNodeView) {
  return Object.values(node.properties).map((value) => String(value ?? ""));
}

function nodeMatchesQuery(node: GraphNodeView, query: string) {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return true;

  return [
    node.nodeCode,
    ...node.labels,
    node.fullText ?? "",
    node.embeddingText ?? "",
    ...propertyValues(node),
  ].some((value) => value.toLowerCase().includes(normalizedQuery));
}

function nodeMatchesLabels(node: GraphNodeView, labels: string[]) {
  return labels.length === 0 || labels.some((label) => node.labels.includes(label));
}

function edgeTouchesNode(edge: GraphEdgeView, nodeCode: string) {
  return edge.sourceNodeCode === nodeCode || edge.targetNodeCode === nodeCode;
}

export function useKnowledgeGraphExplorer(filters: KnowledgeGraphExplorerFilters, selectedNodeCode: string): KnowledgeGraphExplorerView {
  const graph = useServiceQuery<KnowledgeGraphView>({
    apiInitialData: getKnowledgeGraphMock,
    deps: [filters.namespace, filters.workspaceCode],
    domain: "knowledge-graph",
    endpoint: knowledgeGraphApiEndpoints.queryKnowledgeGraph,
    mockData: getKnowledgeGraphMock,
    params: {
      namespace: filters.namespace,
      workspaceCode: filters.workspaceCode,
    },
  });

  return useMemo(() => {
    const labelMatchedNodes = graph.nodes.filter((node) => nodeMatchesLabels(node, filters.labels));
    const resultNodes = labelMatchedNodes.filter((node) => nodeMatchesQuery(node, filters.query));
    const visibleNodeCodes = new Set(resultNodes.map((node) => node.nodeCode));
    const visibleEdges = graph.edges.filter((edge) => (
      visibleNodeCodes.has(edge.sourceNodeCode)
      && visibleNodeCodes.has(edge.targetNodeCode)
      && (filters.relationship === "all" || edge.relationship === filters.relationship)
    ));
    const selectedNode = graph.nodes.find((node) => node.nodeCode === selectedNodeCode)
      ?? resultNodes[0]
      ?? graph.nodes[0];
    const selectedNodeEdges = graph.edges.filter((edge) => edgeTouchesNode(edge, selectedNode.nodeCode));
    const highlightedNodeCodes = new Set<string>([selectedNode.nodeCode]);
    for (const edge of selectedNodeEdges) {
      highlightedNodeCodes.add(edge.sourceNodeCode);
      highlightedNodeCodes.add(edge.targetNodeCode);
    }

    return {
      ...graph,
      highlightedEdgeCodes: selectedNodeEdges.map((edge) => edge.edgeCode),
      highlightedNodeCodes: Array.from(highlightedNodeCodes),
      resultNodes,
      selectedNode,
      selectedNodeEdges,
      visibleEdges,
      visibleNodes: resultNodes,
    };
  }, [
    filters.labels,
    filters.query,
    filters.relationship,
    graph,
    selectedNodeCode,
  ]);
}

export function useKnowledgeGraphByDomain() {
  return getKnowledgeGraphMock();
}
