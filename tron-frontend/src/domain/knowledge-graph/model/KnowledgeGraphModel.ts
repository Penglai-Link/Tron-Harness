export type GraphPropertyValue = string | number | boolean | null;

export interface GraphScopeView {
  appCode: string;
  currentWorkspaceCode: string;
  namedGraph: string;
  namespace: string;
  rootWorkspaceCode: string;
  schema: string;
  userCode: string;
  visibleWorkspaceCodes: string[];
  workspaceCode: string;
}

export interface GraphMetrics {
  edgeCount: number;
  namespaceCount: number;
  nodeCount: number;
}

export interface GraphNodeView {
  createdAt: string;
  embeddingText?: string | null;
  fullText?: string | null;
  labels: string[];
  nodeCode: string;
  properties: Record<string, GraphPropertyValue>;
  score?: number;
  updatedAt: string;
  x: number;
  y: number;
}

export interface GraphEdgeView {
  createdAt: string;
  edgeCode: string;
  embeddingText?: string | null;
  fullText?: string | null;
  properties: Record<string, GraphPropertyValue>;
  relationship: string;
  sourceNodeCode: string;
  targetNodeCode: string;
  updatedAt: string;
}

export interface GraphPathView {
  edgeCodes: string[];
  id: string;
  label: string;
  nodeCodes: string[];
}

export interface GraphFacet {
  count: number;
  label: string;
  value: string;
}

export interface KnowledgeGraphView {
  edges: GraphEdgeView[];
  facets: GraphFacet[];
  metrics: GraphMetrics;
  namespaces: string[];
  nodes: GraphNodeView[];
  paths: GraphPathView[];
  scope: GraphScopeView;
}

export interface KnowledgeGraphExplorerFilters {
  labels: string[];
  namespace: string;
  query: string;
  relationship: string;
  workspaceCode: string;
}

export interface KnowledgeGraphExplorerView extends KnowledgeGraphView {
  highlightedEdgeCodes: string[];
  highlightedNodeCodes: string[];
  resultNodes: GraphNodeView[];
  selectedNode: GraphNodeView;
  selectedNodeEdges: GraphEdgeView[];
  visibleEdges: GraphEdgeView[];
  visibleNodes: GraphNodeView[];
}
