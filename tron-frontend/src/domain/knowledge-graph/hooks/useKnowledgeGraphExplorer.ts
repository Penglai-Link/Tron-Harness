import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import {
  expandKnowledgeGraph,
  fetchKnowledgeGraphNamespaces,
  fetchKnowledgeGraphSnapshot,
  queryKnowledgeGraph,
} from "@/domain/knowledge-graph/api";
import { getKnowledgeGraphMock } from "@/domain/knowledge-graph/mock/data";
import type {
  GraphEdgeView,
  GraphNodeView,
  KnowledgeGraphExplorerFilters,
  KnowledgeGraphExplorerView,
  KnowledgeGraphQueryRequest,
  KnowledgeGraphView,
} from "@/domain/knowledge-graph/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function nodeMatchesLabels(node: GraphNodeView, labels: string[]) {
  return labels.length === 0 || labels.some((label) => node.labels.includes(label));
}

function edgeTouchesNode(edge: GraphEdgeView, nodeCode: string) {
  return edge.sourceNodeCode === nodeCode || edge.targetNodeCode === nodeCode;
}

function createQueryBody(filters: KnowledgeGraphExplorerFilters): KnowledgeGraphQueryRequest {
  return {
    inheritanceMode: "WORKSPACE_LINEAGE",
    labels: filters.labels,
    namespace: filters.namespace,
    query: "",
    relationships: filters.relationships,
    scopeModes: ["SPACE", "USER"],
    workspaceCode: filters.workspaceCode,
  };
}

function mergeGraphWindow(current: KnowledgeGraphView, next: KnowledgeGraphView): KnowledgeGraphView {
  const nodesByCode = new Map(current.nodes.map((node) => [node.nodeCode, node]));
  const edgesByCode = new Map(current.edges.map((edge) => [edge.edgeCode, edge]));

  for (const node of next.nodes) {
    nodesByCode.set(node.nodeCode, node);
  }
  for (const edge of next.edges) {
    edgesByCode.set(edge.edgeCode, edge);
  }

  return {
    ...next,
    edges: Array.from(edgesByCode.values()),
    nodes: Array.from(nodesByCode.values()),
    window: {
      ...next.window,
      returnedEdgeCount: edgesByCode.size,
      returnedNodeCount: nodesByCode.size,
    },
  };
}

export function useKnowledgeGraphNamespaces(workspaceCode: string) {
  const dataSource = getServiceDataSource("knowledge-graph");
  const mockNamespaces = useMemo(() => getKnowledgeGraphMock().namespaces, []);
  const [namespaces, setNamespaces] = useState<string[]>(() => (
    dataSource === "mock" ? mockNamespaces : []
  ));
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (dataSource === "mock") {
      setNamespaces(mockNamespaces);
      setLoading(false);
      return undefined;
    }

    if (!workspaceCode.trim()) {
      setNamespaces([]);
      setLoading(false);
      return undefined;
    }

    const controller = new AbortController();
    setNamespaces([]);
    setLoading(true);
    fetchKnowledgeGraphNamespaces({
      workspaceCode,
    }, controller.signal)
      .then((nextNamespaces) => {
        if (controller.signal.aborted) return;
        setNamespaces(nextNamespaces);
        setLoading(false);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setNamespaces(mockNamespaces);
        setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, mockNamespaces, workspaceCode]);

  return {
    loading,
    namespaces,
  };
}

function useApiKnowledgeGraph(filters: KnowledgeGraphExplorerFilters, enabled: boolean, refreshToken: number) {
  const [graph, setGraph] = useState<KnowledgeGraphView>(() => getKnowledgeGraphMock());
  const [filterOptionGraph, setFilterOptionGraph] = useState<KnowledgeGraphView>(() => getKnowledgeGraphMock());
  const [loading, setLoading] = useState(false);
  const filterKey = JSON.stringify({
    labels: filters.labels,
    relationships: filters.relationships,
  });
  const previousFilterKey = useRef<string | null>(null);

  useEffect(() => {
    if (!enabled) return undefined;

    const controller = new AbortController();

    setLoading(true);
    fetchKnowledgeGraphSnapshot({
      namespace: filters.namespace,
      workspaceCode: filters.workspaceCode,
    }, controller.signal)
      .then((snapshot) => {
        if (controller.signal.aborted) return;
        setGraph(snapshot);
        setFilterOptionGraph(snapshot);
        setLoading(false);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        const fallbackGraph = getKnowledgeGraphMock();
        setGraph(fallbackGraph);
        setFilterOptionGraph(fallbackGraph);
        setLoading(false);
      });

    return () => controller.abort();
  }, [enabled, filters.namespace, filters.workspaceCode, refreshToken]);

  useEffect(() => {
    if (!enabled) return undefined;

    if (previousFilterKey.current === null) {
      previousFilterKey.current = filterKey;
      return undefined;
    }
    if (previousFilterKey.current === filterKey) return undefined;

    previousFilterKey.current = filterKey;
    const controller = new AbortController();

    setLoading(true);
    queryKnowledgeGraph(createQueryBody(filters), controller.signal)
      .then((result) => {
        if (controller.signal.aborted) return;
        setGraph(result);
        setLoading(false);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setGraph(getKnowledgeGraphMock());
        setLoading(false);
      });

    return () => controller.abort();
  }, [enabled, filterKey, filters]);

  const expandNode = useCallback(async (nodeCode: string) => {
    const result = await expandKnowledgeGraph({
      depth: 1,
      inheritanceMode: "WORKSPACE_LINEAGE",
      limit: 100,
      namespace: filters.namespace,
      nodeCode,
      relationships: [],
      scopeModes: ["SPACE", "USER"],
      workspaceCode: filters.workspaceCode,
    });
    setGraph((current) => mergeGraphWindow(current, result));
  }, [filters.namespace, filters.workspaceCode]);

  return {
    expandNode,
    filterOptionGraph,
    graph,
    loading,
  };
}

export function useKnowledgeGraphExplorer(
  filters: KnowledgeGraphExplorerFilters,
  selectedNodeCode: string,
  refreshToken = 0,
): KnowledgeGraphExplorerView {
  const dataSource = getServiceDataSource("knowledge-graph");
  const apiEnabled = dataSource === "api"
    && filters.namespace.trim().length > 0
    && filters.workspaceCode.trim().length > 0;
  const apiGraph = useApiKnowledgeGraph(filters, apiEnabled, refreshToken);
  const mockGraph = useMemo(() => getKnowledgeGraphMock(), []);
  const graph = dataSource === "api" ? apiGraph.graph : mockGraph;
  const filterOptionGraph = dataSource === "api" ? apiGraph.filterOptionGraph : mockGraph;
  const expandNode = dataSource === "api"
    ? apiGraph.expandNode
    : async () => undefined;
  const loading = dataSource === "api" ? apiGraph.loading : false;

  return useMemo(() => {
    const isMockMode = dataSource === "mock";
    const resultNodes = isMockMode
      ? graph.nodes
        .filter((node) => nodeMatchesLabels(node, filters.labels))
      : graph.nodes;
    const visibleNodeCodes = new Set(resultNodes.map((node) => node.nodeCode));
    const visibleEdges = isMockMode
      ? graph.edges.filter((edge) => (
        visibleNodeCodes.has(edge.sourceNodeCode)
        && visibleNodeCodes.has(edge.targetNodeCode)
        && (filters.relationships.length === 0 || filters.relationships.includes(edge.relationship))
      ))
      : graph.edges;
    const selectedNode = selectedNodeCode
      ? graph.nodes.find((node) => node.nodeCode === selectedNodeCode) ?? null
      : null;
    const selectedNodeEdges = selectedNode
      ? graph.edges.filter((edge) => edgeTouchesNode(edge, selectedNode.nodeCode))
      : [];
    const highlightedNodeCodes = new Set<string>(selectedNode ? [selectedNode.nodeCode] : []);
    for (const edge of selectedNodeEdges) {
      highlightedNodeCodes.add(edge.sourceNodeCode);
      highlightedNodeCodes.add(edge.targetNodeCode);
    }

    return {
      ...graph,
      expandNode,
      filterOptionEdges: filterOptionGraph.edges,
      filterOptionFacets: filterOptionGraph.facets,
      filterOptionNodes: filterOptionGraph.nodes,
      highlightedEdgeCodes: selectedNodeEdges.map((edge) => edge.edgeCode),
      highlightedNodeCodes: Array.from(highlightedNodeCodes),
      loading,
      resultNodes,
      selectedNode,
      selectedNodeEdges,
      visibleEdges,
      visibleNodes: resultNodes,
      window: isMockMode
        ? {
          ...graph.window,
          returnedEdgeCount: visibleEdges.length,
          returnedNodeCount: resultNodes.length,
        }
        : graph.window,
    };
  }, [
    filters.labels,
    filters.relationships,
    graph,
    filterOptionGraph.edges,
    filterOptionGraph.facets,
    filterOptionGraph.nodes,
    dataSource,
    expandNode,
    loading,
    selectedNodeCode,
  ]);
}

export function useKnowledgeGraphByDomain() {
  return getKnowledgeGraphMock();
}
