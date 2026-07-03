export type GraphPropertyValue = string | number | boolean | null;

export type GraphLayoutMode = "Radial" | "Smart" | "Force" | "Hierarchy" | "Grid";

export interface GraphScopeView {
  appCode: string;
  currentWorkspaceCode: string;
  inheritanceMode: "WORKSPACE_LINEAGE";
  namedGraph: string;
  namespace: string;
  rootWorkspaceCode: string;
  schema: string;
  scopeModes: Array<"SPACE" | "USER">;
  userCode: string;
  visibleUserCodes: string[];
  visibleWorkspaceCodes: string[];
  workspaceCode: string;
}

export interface GraphWindowView {
  edgeTotal: number;
  hasMore: boolean;
  limit: number;
  nodeTotal: number;
  returnedEdgeCount: number;
  returnedNodeCount: number;
}

export interface GraphMetrics {
  edgeCount: number;
  namespaceCount: number;
  nodeCount: number;
}

export interface GraphNodeView {
  createdAt: string;
  embeddingText?: string | null;
  editable: boolean;
  fullText?: string | null;
  inherited: boolean;
  labels: string[];
  nodeCode: string;
  originUserCode?: string | null;
  originWorkspaceCode: string;
  originWorkspaceName?: string | null;
  properties: Record<string, GraphPropertyValue>;
  score?: number;
  scopeType: "SPACE" | "USER";
  updatedAt: string;
  x?: number;
  y?: number;
}

export interface GraphEdgeView {
  createdAt: string;
  edgeCode: string;
  embeddingText?: string | null;
  editable: boolean;
  fullText?: string | null;
  inherited: boolean;
  originUserCode?: string | null;
  originWorkspaceCode: string;
  originWorkspaceName?: string | null;
  properties: Record<string, GraphPropertyValue>;
  relationship: string;
  scopeType: "SPACE" | "USER";
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
  window: GraphWindowView;
}

export interface KnowledgeGraphExplorerFilters {
  labels: string[];
  namespace: string;
  relationships: string[];
  workspaceCode: string;
}

export interface KnowledgeGraphExplorerView extends KnowledgeGraphView {
  expandNode: (nodeCode: string) => Promise<void>;
  filterOptionEdges: GraphEdgeView[];
  filterOptionFacets: GraphFacet[];
  filterOptionNodes: GraphNodeView[];
  highlightedEdgeCodes: string[];
  highlightedNodeCodes: string[];
  loading: boolean;
  resultNodes: GraphNodeView[];
  selectedNode: GraphNodeView | null;
  selectedNodeEdges: GraphEdgeView[];
  visibleEdges: GraphEdgeView[];
  visibleNodes: GraphNodeView[];
}

export interface KnowledgeGraphQueryRequest {
  inheritanceMode: "WORKSPACE_LINEAGE";
  labels: string[];
  namespace: string;
  query: string;
  relationships: string[];
  scopeModes: Array<"SPACE" | "USER">;
  workspaceCode: string;
}

export interface KnowledgeGraphExpandRequest {
  depth: number;
  inheritanceMode: "WORKSPACE_LINEAGE";
  limit: number;
  namespace: string;
  nodeCode: string;
  relationships: string[];
  scopeModes: Array<"SPACE" | "USER">;
  workspaceCode: string;
}
