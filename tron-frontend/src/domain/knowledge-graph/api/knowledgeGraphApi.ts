import type {
  KnowledgeGraphExpandRequest,
  KnowledgeGraphQueryRequest,
  KnowledgeGraphView,
} from "@/domain/knowledge-graph/model";
import { requestApi } from "@/shared/api/httpClient";

export const knowledgeGraphApiEndpoints = {
  expandKnowledgeGraph: "POST /api/v1/knowledge-graph/expand",
  listKnowledgeGraphNamespaces: "GET /api/v1/knowledge-graph/list-namespaces",
  queryKnowledgeGraph: "POST /api/v1/knowledge-graph/query",
  snapshotKnowledgeGraph: "GET /api/v1/knowledge-graph",
} as const;

export interface KnowledgeGraphNamespaceParams {
  workspaceCode: string;
}

export interface KnowledgeGraphSnapshotParams {
  namespace: string;
  workspaceCode: string;
}

export function fetchKnowledgeGraphNamespaces(
  params: KnowledgeGraphNamespaceParams,
  signal?: AbortSignal,
) {
  return requestApi<string[]>(knowledgeGraphApiEndpoints.listKnowledgeGraphNamespaces, {
    queryParams: {
      "workspace-code": params.workspaceCode,
    },
    signal,
  });
}

export function fetchKnowledgeGraphSnapshot(
  params: KnowledgeGraphSnapshotParams,
  signal?: AbortSignal,
) {
  return requestApi<KnowledgeGraphView>(knowledgeGraphApiEndpoints.snapshotKnowledgeGraph, {
    queryParams: {
      namespace: params.namespace,
      "workspace-code": params.workspaceCode,
    },
    signal,
  });
}

export function queryKnowledgeGraph(
  body: KnowledgeGraphQueryRequest,
  signal?: AbortSignal,
) {
  return requestApi<KnowledgeGraphView>(knowledgeGraphApiEndpoints.queryKnowledgeGraph, {
    body,
    signal,
  });
}

export function expandKnowledgeGraph(
  body: KnowledgeGraphExpandRequest,
  signal?: AbortSignal,
) {
  return requestApi<KnowledgeGraphView>(knowledgeGraphApiEndpoints.expandKnowledgeGraph, {
    body,
    signal,
  });
}
